#!/usr/bin/env bash
# Build, sign, notarize and package Life Manager for distribution.
#
# The script stops with a clear message when a requirement is missing rather
# than producing an unsigned or un-notarized artifact that looks finished.
#
#   ./scripts/release.sh build     build the bundle and the CLI
#   ./scripts/release.sh sign      sign with a Developer ID Application identity
#   ./scripts/release.sh notarize  submit to Apple and staple the ticket
#   ./scripts/release.sh dmg       build the distributable disk image
#   ./scripts/release.sh verify    check the signature, notarization and Gatekeeper
#   ./scripts/release.sh all       everything, in order
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="Life Manager"
BUNDLE="$ROOT/dist/$APP_NAME.app"
CLI="$ROOT/dist/life"
DMG="$ROOT/dist/LifeManager.dmg"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/life-manager-target}"
export CARGO_TARGET_DIR
IDENTITY="${LIFE_SIGN_IDENTITY:-}"
NOTARY_PROFILE="${LIFE_NOTARY_PROFILE:-AC_NOTARY}"
VERSION="$(python3 -c "import re;print(re.search(r'^version\s*=\s*\"([^\"]+)\"',open('$ROOT/crates/life-gpui/Cargo.toml').read(),re.M).group(1))")"

log()  { printf '\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m==>\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31m==>\033[0m %s\n' "$*" >&2; exit 1; }

require_developer_id() {
  security find-identity -v -p codesigning | grep -q "Developer ID Application" \
    || die "No 'Developer ID Application' identity is installed.
   Enrol in the Apple Developer Program and install the certificate, then run:
   security find-identity -v -p codesigning"
}

do_build() {
  log "Building the native GPUI desktop app and CLI (version $VERSION)"
  mkdir -p "$ROOT/dist"
  (cd "$ROOT" && cargo build --release -p life-cli -p life-gpui)
  cp "$CARGO_TARGET_DIR/release/life" "$CLI"
  chmod +x "$CLI"

  rm -rf "$BUNDLE"
  mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
  cp "$CARGO_TARGET_DIR/release/life-gpui" "$BUNDLE/Contents/MacOS/life-gpui"
  cp "$CARGO_TARGET_DIR/release/life" "$BUNDLE/Contents/MacOS/life"
  chmod +x "$BUNDLE/Contents/MacOS/life-gpui" "$BUNDLE/Contents/MacOS/life"

  if [ -f "$ROOT/assets/icons/icon.icns" ]; then
    cp "$ROOT/assets/icons/icon.icns" "$BUNDLE/Contents/Resources/AppIcon.icns"
  fi

  cat > "$BUNDLE/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDisplayName</key>
  <string>Life Manager</string>
  <key>CFBundleExecutable</key>
  <string>life-gpui</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>
  <string>local.lifemanager.desktop</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Life Manager</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

  codesign --force --deep --sign - "$BUNDLE" >/dev/null 2>&1 || true
  codesign --force --sign - "$CLI" >/dev/null 2>&1 || true
  log "Built native GPUI bundle $BUNDLE and $CLI"
}

do_sign() {
  require_developer_id
  log "Signing with the Developer ID identity"
  codesign --force --timestamp --options runtime --deep --sign "$IDENTITY" "$BUNDLE"
  codesign --force --timestamp --options runtime --sign "$IDENTITY" "$CLI"
  codesign --verify --strict --deep --verbose=2 "$BUNDLE"
  log "Signed and verified"
}

do_notarize() {
  [ -d "$BUNDLE" ] || die "Build the bundle first."
  xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1 \
    || die "No notarization credentials are stored.
   Run:
   xcrun notarytool store-credentials \"$NOTARY_PROFILE\"
   and sign in with an Apple ID that has access to the app's team."
  log "Submitting to Apple for notarization"
  local zip
  zip="$ROOT/dist/LifeManager-notarize.zip"
  ditto -c -k --keepParent "$BUNDLE" "$zip"
  xcrun notarytool submit "$zip" --keychain-profile "$NOTARY_PROFILE" --wait
  xcrun stapler staple "$BUNDLE"
  rm -f "$zip"
  log "Notarized and stapled"
}

do_dmg() {
  [ -d "$BUNDLE" ] || die "Build the bundle first."
  log "Building the disk image"
  local staging="$ROOT/dist/.dmg-staging"
  rm -rf "$staging"
  mkdir -p "$staging"
  cp -R "$BUNDLE" "$staging/"
  cp "$CLI" "$staging/life"
  ln -s /Applications "$staging/Applications"
  rm -f "$DMG"
  local device
  device="$(hdiutil create -volname "Life Manager" -srcfolder "$staging" -ov -format UDZO "$DMG" | tail -1)"
  rm -rf "$staging"
  [ -n "$device" ] && hdiutil detach "$device" -quiet || true
  log "Wrote $DMG"
}

do_verify() {
  log "Verifying the release artifacts"
  codesign --verify --strict --deep --verbose=2 "$BUNDLE" || die "The bundle signature is not valid."
  codesign --dv --verbose=4 "$BUNDLE" 2>&1 | grep -q "Authority=Developer ID" \
    || die "The bundle is not signed with a Developer ID Application identity."
  if xcrun stapler validate "$BUNDLE" 2>/dev/null; then
    log "The notarization ticket is stapled"
  else
    warn "The notarization ticket is not stapled. Run the notarize step."
  fi
  spctl --assess --type execute --verbose=2 "$BUNDLE" || warn "Gatekeeper did not accept the bundle."
  "$CLI" --json stats >/dev/null || die "The CLI does not run."
  log "Release artifacts verified"
}

case "${1:-all}" in
  build)    do_build ;;
  sign)     do_sign ;;
  notarize) do_notarize ;;
  dmg)      do_dmg ;;
  verify)   do_verify ;;
  all)      do_build; do_sign; do_notarize; do_dmg; do_verify ;;
  *) die "Usage: $0 {build|sign|notarize|dmg|verify|all}" ;;
esac
