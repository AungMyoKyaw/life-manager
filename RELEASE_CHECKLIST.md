# Release checklist

Run this before publishing. Every line must be observed, not assumed.

**Current state: this build is not publishable.** It is ad-hoc signed, `spctl --assess --type execute` rejects it, and it carries no notarization ticket. Sections 2 and 3 below are the gate. Nothing in section 6 can happen until they pass.

## 1. Build

```bash
export CARGO_TARGET_DIR=/tmp/life-manager-target
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --offline
python3 qa/full_e2e.py
python3 qa/figcraft_parity_e2e.py
python3 qa/v9_extensions_e2e.py
python3 qa/v11_reminders_habits_e2e.py
python3 qa/parity_v15_e2e.py
python3 qa/release_e2e.py
python3 qa/encryption_e2e.py
python3 qa/board_coverage.py
python3 design/scripts/validate_design.py
```

Note: `qa/v9_extensions_e2e.py` and `qa/v11_reminders_habits_e2e.py` default `LIFE_CLI` to `dist/life` (run `./scripts/release.sh build` or set `LIFE_CLI=/tmp/life-manager-target/release/life`).

Expected: no failures anywhere. The design validation must report `PASS`.

## 2. Identity

A Developer ID Application certificate is required. It cannot be created by a script.

```bash
security find-identity -v -p codesigning | grep "Developer ID Application"
```

If nothing is listed, enrol at <https://developer.apple.com/account/resources/certificates/list> and install the certificate into the login keychain.

## 3. Notarization credentials

```bash
xcrun notarytool store-credentials "AC_NOTARY"
```

Sign in with an Apple ID that has access to the team. The password is stored in the keychain, never in the repository.

## 4. Sign, notarize and package

```bash
export LIFE_SIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)"
./scripts/release.sh all
```

The script builds, signs with a timestamp and the hardened runtime, submits to Apple, waits for the result, staples the ticket, writes the disk image and verifies everything.

## 5. Verify the notarized artifact

```bash
codesign --verify --strict --deep --verbose=2 "dist/Life Manager.app"
xcrun stapler validate "dist/Life Manager.app"
spctl --assess --type execute --verbose=2 "dist/Life Manager.app"
```

`spctl` must accept the bundle. A bundle that is only ad-hoc signed will be rejected, and that is the expected result before you have a Developer ID.

## 6. Publish

```bash
shasum -a 256 dist/LifeManager.dmg
./scripts/install-homebrew.sh
# put the sha256 into homebrew/life-manager.rb

gh release create "v$(python3 -c "import re;print(re.search(r'^version\s*=\s*\"([^\"]+)\"',open('crates/life-gpui/Cargo.toml').read(),re.M).group(1))")" \
  dist/LifeManager.dmg \
  "dist/Life Manager.app" \
  --title "Life Manager" \
  --notes-file CHANGELOG.md
```

Attach the disk image and, optionally, the raw application bundle. The Homebrew cask points at the disk image asset, so the sha256 must match the uploaded file.

## 7. After the release

- Verify a clean install on a second machine: drag the app to Applications, launch it, complete onboarding, create a task, then confirm `life task list --json` shows the same task.
- Confirm `brew install --cask life-manager` works from the published cask.
- Update `CHANGELOG.md` with the published version and verification summary.

## What is still outside the release

These are honest boundaries, not defects to hide:

- The GitHub, Google Calendar, CalDAV, and Apple Calendar integrations (`integrations.configure`) currently check `LIFE_*` environment variables and record status in `integration_accounts`; macOS Keychain token storage and live OAuth/CalDAV/EventKit sync are not yet implemented.
- The background reminder worker delivers through Notification Center, which the user must authorise.
- The database is unencrypted by default; optional SQLCipher + Argon2id encryption at rest is available via `life db encrypt`.
