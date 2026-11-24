#!/usr/bin/env bash
# Publish the Homebrew cask for Life Manager.
#
# The cask points at the GitHub release asset, so the repository must have a
# release tagged with the version and the DMG attached to it.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="${LIFE_REPO:?Set LIFE_REPO to the public owner/repository before generating a Homebrew cask}"
VERSION="$(python3 -c "import re;print(re.search(r'^version\s*=\s*\"([^\"]+)\"',open('$ROOT/crates/life-gpui/Cargo.toml').read(),re.M).group(1))")"
CASKS="$ROOT/homebrew"
mkdir -p "$CASKS"

cat > "$CASKS/life-manager.rb" <<RUBY
cask "life-manager" do
  version "$VERSION"
  sha256 "REPLACE_WITH_THE_SHA256_OF_THE_DMG"

  url "https://github.com/$REPO/releases/download/v#{version}/LifeManager.dmg"
  name "Life Manager"
  desc "Local-first productivity app for tasks, calendar, focus, habits, and goals"
  homepage "https://github.com/$REPO"

  livecheck do
    url :url
    regex(/^v?(\\d+(?:\\.\\d+)+)$/i)
  end

  depends_on arch: :arm64
  depends_on macos: :ventura

  app "Life Manager.app"
  binary "#{appdir}/Life Manager.app/Contents/MacOS/life"

  postflight_steps do
    run "/usr/bin/xattr",
        args:         ["-dr", "com.apple.quarantine", "{{appdir}}/Life Manager.app"],
        must_succeed: false
  end

  zap trash: [
    "~/Library/Application Support/LifeManager",
    "~/Library/Preferences/local.lifemanager.desktop.plist",
    "~/Library/Saved Application State/local.lifemanager.desktop.savedState",
  ]
end
RUBY

echo "Wrote $CASKS/life-manager.rb"
echo "Replace the sha256 line with: shasum -a 256 dist/LifeManager.dmg"
