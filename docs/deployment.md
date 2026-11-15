# Building and releasing

## Local builds

Prerequisites: Rust stable, Node.js 22, pnpm 9 and the [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/). On Debian/Ubuntu:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev \
  patchelf libxdo-dev libssl-dev libdbus-1-dev
```

```bash
pnpm install
pnpm tauri build                 # installers in src-tauri/target/release/bundle/
pnpm tauri build --target aarch64-apple-darwin   # explicit target (rustup target add … first)
```

`bundle.targets` is `"all"`, so a build produces:

| OS | Artifacts |
|---|---|
| macOS | `.app`, `.dmg` |
| Windows | `.msi` (WiX), `-setup.exe` (NSIS) |
| Linux | `.AppImage`, `.deb`, `.rpm` |

### Updater artifacts

Updater bundles and signatures are produced only when the release overlay `src-tauri/tauri.release.conf.json` is used. It sets `bundle.createUpdaterArtifacts: true`. The updater signing key must also be in the environment; `.env` files are not read.

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/clipsa-updater.key)"   # path or contents
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""                            # if set
pnpm tauri build --config src-tauri/tauri.release.conf.json
```

The public half of this key is `plugins.updater.pubkey` in `tauri.conf.json`. **Back up the private key.** If it is lost, existing installations can never auto-update again. Only generate a new pair (`pnpm tauri signer generate -w ~/.tauri/clipsa-updater.key`) if you accept that existing users must reinstall manually.

## CI (`.github/workflows/ci.yml`)

Runs on pushes and PRs to `master` on Ubuntu, macOS and Windows: `pnpm check`, `pnpm build` (`tauri::generate_context!` needs `build/` to exist), `cargo fmt --check`, `cargo clippy -D warnings` (plain, `self-update` and, on macOS, `app-store`), `cargo test`. Tests use injected keys and never touch the OS keychain. Changes that only touch `site/`, `docs/` or Markdown files skip CI.

## Release workflow (`.github/workflows/release.yml`)

Triggered by pushing a `v*` tag, or manually with `workflow_dispatch`.

1. **notes**: extracts the `CHANGELOG.md` section for the version in `tauri.conf.json` (see [Release notes](#release-notes)). The workflow stops here, before anything is built or uploaded, if the section is missing or empty.
2. **build** (matrix):
   - macOS arm64: `macos-latest`, `--target aarch64-apple-darwin`
   - macOS x64: `macos-latest`, `--target x86_64-apple-darwin`
   - Linux: `ubuntu-22.04`
   - Windows: `windows-latest`

   Each job runs `tauri-apps/tauri-action@v0` with `--config src-tauri/tauri.release.conf.json`. It uploads installers, updater bundles and `.sig` files to a **draft** release `v<version>`, where the version comes from `tauri.conf.json`, and merges each platform's entry into `latest.json`. The release notes are passed as `releaseBody`, which tauri-action uses both as the GitHub release text and as the `notes` field of `latest.json`.
3. **publish**: runs only if every build succeeded and makes the draft public (`gh release edit --draft=false --latest`). From that moment `https://github.com/mordamax/clipsa.dev/releases/latest/download/latest.json` serves the new version to the in-app updater.

If any build fails, the release stays a draft and no user receives the update. Fix the problem, delete the draft, and re-run.

### Release notes

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Every version has a section headed `## [X.Y.Z] - YYYY-MM-DD`; while it is in progress, write `## [X.Y.Z] - Unreleased` and replace `Unreleased` with the date when you release (the workflow does not check the date). Group entries under `### Added`, `### Changed`, `### Fixed`, `### Removed` or `### Security`.

Everything between the version's heading and the next `## [` heading is published unchanged: as the GitHub release text, and in Clipsa's "Software Update" window (direct-download builds), which shows `latest.json`'s `notes` under "Release Notes". Write it for users, not for developers. The window renders a small Markdown subset: `##`/`###` headings, `- ` bullets, paragraphs, `**bold**` and `` `code` ``. Links are shown as their text only, and any HTML is displayed literally.

Notes flow: `CHANGELOG.md` → **notes** job → tauri-action `releaseBody` → GitHub release text and `latest.json` `notes` → updater plugin `Update.body` → update window.

### Cutting a release

1. Set the same version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`, and make sure `CHANGELOG.md` has a non-empty section for it (dated). Commit to `master`.
2. `git tag vX.Y.Z && git push origin vX.Y.Z`. The tag must equal `v` + the version in `tauri.conf.json`.
3. Wait for the workflow to finish, then check the published release notes and assets.

### Secrets

| Secret | Needed for |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | **Required.** Contents of `~/.tauri/clipsa-updater.key` from the maintainer machine. Without it, building with updater artifacts fails. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Only if the updater key is password-protected |
| `APPLE_CERTIFICATE` | Required. macOS code signing: base64 of the Developer ID Application `.p12` (`openssl base64 -A -in cert.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | Required. Password of that `.p12` |
| `APPLE_SIGNING_IDENTITY` | Optional; defaults to the identity in the certificate |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | Required. Notarization (Apple ID email, app-specific password, team id) |
| `HOMEBREW_TAP_TOKEN` | Optional. Fine-grained token with contents:write on `mordamax/homebrew-tap`; the `homebrew` job pushes the rendered cask there. Without it the cask is only attached to the release. |

macOS releases must be signed with a Developer ID and notarized; the workflow fails before building if any required Apple secret is missing. A stable signing identity is what lets users click "Always Allow" for keychain access and grant Accessibility once: macOS ties both to the code signature, so unsigned builds would re-prompt after every update. (The Tauri bundler treats an Apple variable set to an empty string as configured, so the workflow exports them only after this check.)

Windows installers are not code-signed; SmartScreen may warn until the app builds reputation.

## Website (Vercel)

The static site in `site/` is deployed to the Vercel project `makhq/clipsa`, whose root directory is `site/` (no framework, no build or install step). `site/vercel.json` sets clean URLs and security headers (CSP without any third-party origin, `no-referrer`, `nosniff`). The repo-root `.vercelignore` uploads only `site/` for CLI deploys.

- Deploy manually: `npx vercel@latest deploy --prod --scope makhq` from the repo root (after `vercel link --project clipsa --scope makhq` once).
- Automatic deploys: connect the GitHub repository to the project in the Vercel dashboard (Settings → Git); pushes to `master` then deploy `site/`.
- Keep Vercel Web Analytics and Speed Insights off; the privacy policy promises no trackers.

## Mac App Store

The App Store edition is built from the same code with the overlay `src-tauri/tauri.appstore.conf.json`. It sets `build.features: ["app-store"]`, which replaces the base `["self-update"]`, so the updater plugin is not compiled in. It also sets `bundle.category: "Utility"`, `minimumSystemVersion: "13.0"`, the sandbox entitlements, the provisioning profile and `src-tauri/appstore/Info.plist`, which is merged into the bundle's Info.plist. The upload workflow is `.github/workflows/appstore.yml`. It follows the [Tauri App Store guide](https://v2.tauri.app/distribute/app-store/).

### Edition differences

| | Direct download (GitHub / Homebrew) | Mac App Store |
|---|---|---|
| Cargo feature | `self-update` | `app-store` |
| Updates | Built-in updater (opt-in) | App Store |
| Picker `Enter` | Pastes into the previous app (needs Accessibility) | Copies the item and hides the picker; the user presses ⌘V. The sandbox does not allow sending keystrokes to other apps. `paste_item` is compiled to a rejection, `check_permissions` reports Accessibility as `not_required` (the Accessibility UI is hidden), and the picker footer reads "↵ copy, then ⌘V to paste". |
| Launch at login | `tauri-plugin-autostart` LaunchAgent with `--launched-at-login` | `SMAppService.mainAppService` (ServiceManagement, macOS 13+). The autostart plugin is not registered because the sandbox blocks its LaunchAgent and AppleScript writes. If the user has to approve the login item, the setting fails with a message pointing to System Settings → General → Login Items. |
| Start in the tray | Only when started by the login item | A login item cannot pass arguments, so whenever launch at login is on (and registered with macOS) and the first-run screen is done, Clipsa starts in the menu bar. Opening it again (Finder, Dock, Spotlight) shows the window. |
| Data location | `~/Library/Application Support/clipsa` | `~/Library/Containers/com.clipsa.app/Data/Library/Application Support/clipsa`. Separate from the direct edition; history is not migrated. |

Everything else, including capture of text, rich text, images and file lists, encryption, the keychain key, app lock, exclusions and the global hotkey, is the same code.

### Entitlements

`src-tauri/appstore/Entitlements.plist.template` contains:

- `com.apple.security.app-sandbox`: required for the App Store.
- `com.apple.security.network.client`: required by WKWebView. Without it the sandboxed WebContent processes crash at launch and every window stays blank. This was observed locally (see below). Clipsa itself makes no network requests: the CSP only allows `ipc:`, and this edition has no updater.
- `com.apple.application-identifier` = `$(TEAM_ID).com.clipsa.app` and `com.apple.developer.team-identifier` = `$(TEAM_ID)`.

The workflow replaces `$(TEAM_ID)` with the `APPLE_TEAM_ID` secret and writes `src-tauri/appstore/Entitlements.plist`. It also decodes the provisioning profile to `src-tauri/appstore/embedded.provisionprofile`. Both files are gitignored. To build a store bundle by hand, create both files the same way (`sed "s/\$(TEAM_ID)/ABCDE12345/g" Entitlements.plist.template > Entitlements.plist`).

### Sandbox smoke test (local, unsigned)

A debug build uses a key file instead of the keychain, so you can run it in the sandbox without any certificates. Quit the installed Clipsa first; Clipsa runs as a single instance.

```bash
cat > /tmp/sandbox.entitlements <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>com.apple.security.app-sandbox</key><true/>
  <key>com.apple.security.network.client</key><true/>
</dict></plist>
EOF
pnpm tauri build --debug --bundles app --config src-tauri/tauri.appstore.conf.json \
  --config '{"bundle":{"macOS":{"entitlements":"/tmp/sandbox.entitlements","files":{"embedded.provisionprofile":null}}}}'
codesign --force --deep --sign - --entitlements /tmp/sandbox.entitlements src-tauri/target/debug/bundle/macos/Clipsa.app
open -n src-tauri/target/debug/bundle/macos/Clipsa.app --stdout /tmp/clipsa.log
printf 'sandbox-test' | pbcopy
sqlite3 ~/Library/Containers/com.clipsa.app/Data/Library/Application\ Support/clipsa-dev/clipboard.db \
  'SELECT content_type, size, created_at FROM clipboard_items ORDER BY created_at_timestamp DESC LIMIT 5'
# When done: quit it and remove the test bundle so Launchpad/Spotlight don't
# list a second "Clipsa".
pkill -f "target/debug/bundle/macos/Clipsa.app" || true
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -u src-tauri/target/debug/bundle/macos/Clipsa.app
rm -rf src-tauri/target/debug/bundle/macos/Clipsa.app
```

Verified this way: the app and both webviews start, data is written to the container, and text, image and file-list copies are captured. Not covered by this test: the keychain (release builds only), `SMAppService` registration (needs a properly signed app), and the global hotkey and picker interaction.

### One-time setup

1. **App ID.** In [Certificates, Identifiers & Profiles → Identifiers](https://developer.apple.com/account/resources/identifiers/list), register an explicit App ID with Bundle ID `com.clipsa.app` (`tauri.conf.json > identifier`). The App ID Prefix is your team ID.
2. **App record.** In [App Store Connect](https://appstoreconnect.apple.com/apps), create a macOS app with that bundle ID.
3. **Certificates.** Create an **Apple Distribution** certificate and a **Mac Installer Distribution** certificate. Export each with its private key from Keychain Access as a `.p12`.
4. **Provisioning profile.** In Profiles, create a **Mac App Store Connect** profile (Distribution → Mac App Store Connect) for the App ID and the Apple Distribution certificate, then download the `.provisionprofile`.
5. **API key.** In App Store Connect → Users and Access → Integrations → App Store Connect API, create a team key with the App Manager role. Note the Key ID and Issuer ID, and download `AuthKey_<KEYID>.p8`. It can be downloaded only once.
6. Add the secrets below.

### Secrets

The workflow fails before building and lists any missing secret. These are separate from the Developer ID secrets that `release.yml` uses.

| Secret | Value |
|---|---|
| `APPSTORE_CERTIFICATE` | base64 of the Apple Distribution `.p12` (`openssl base64 -A -in dist.p12`). It is passed to the Tauri bundler as `APPLE_CERTIFICATE`. |
| `APPSTORE_CERTIFICATE_PASSWORD` | Password of that `.p12` |
| `APPSTORE_SIGNING_IDENTITY` | e.g. `Apple Distribution: Your Name (ABCDE12345)` |
| `APPSTORE_INSTALLER_CERTIFICATE` | base64 of the Mac Installer Distribution `.p12` |
| `APPSTORE_INSTALLER_CERTIFICATE_PASSWORD` | Password of that `.p12` |
| `APPSTORE_INSTALLER_IDENTITY` | e.g. `3rd Party Mac Developer Installer: Your Name (ABCDE12345)` (the certificate's common name) |
| `APPSTORE_PROVISIONING_PROFILE` | base64 of the `.provisionprofile` |
| `APPLE_TEAM_ID` | Team ID, shared with `release.yml` |
| `APPSTORE_API_KEY_ID` | App Store Connect API Key ID. Required only when uploading. |
| `APPSTORE_API_ISSUER_ID` | API Issuer ID. Required only when uploading. |
| `APPSTORE_API_PRIVATE_KEY` | Contents of `AuthKey_<KEYID>.p8`. Required only when uploading. |

### Running the workflow

Actions → **Mac App Store** → Run workflow. It takes these inputs:

- `build_number`: `CFBundleVersion`. It must be unique for each version in App Store Connect. The default is the workflow run number.
- `upload`: when off, the workflow builds and signs but only attaches `Clipsa.pkg` to the run.

Steps:

1. Write the entitlements and the provisioning profile.
2. Import the installer certificate into a temporary keychain.
3. Run `pnpm tauri build --bundles app --target universal-apple-darwin --config src-tauri/tauri.appstore.conf.json`, signed with the Apple Distribution identity. No notarization variables are set: App Store builds are not notarized.
4. Verify the signature, entitlements, profile, architectures and Info.plist.
5. Run `xcrun productbuild --sign "$APPLE_INSTALLER_IDENTITY" --component Clipsa.app /Applications Clipsa.pkg`.
6. Run `xcrun altool --upload-app --type macos --apiKey … --apiIssuer …`. The key is written to `~/.appstoreconnect/private_keys/AuthKey_<id>.p8` and removed afterwards.

After processing, the build appears under TestFlight in App Store Connect. The marketing version (`CFBundleShortVersionString`) is the `version` in `tauri.conf.json`.

### Export compliance

**This is a legal declaration by the publisher. The owner must confirm it before the first submission.**

`src-tauri/appstore/Info.plist` sets `ITSAppUsesNonExemptEncryption` to `false`, so App Store Connect does not ask the encryption questions for every build. The reasoning:

- Clipsa encrypts only the user's own clipboard history at rest. It uses AES-256-GCM, HKDF-SHA256 and HMAC-SHA256 from the open-source RustCrypto crates (`aes-gcm`, `hkdf`, `hmac`, `sha2`), plus the macOS Keychain for the key. These are standard published algorithms, not proprietary cryptography. The App Store edition makes no network connections.
- Apple says to set the key to `NO` when the app "only uses forms of encryption that are exempt from export compliance documentation requirements" ([Complying with Encryption Export Regulations](https://developer.apple.com/documentation/security/complying-with-encryption-export-regulations), [ITSAppUsesNonExemptEncryption](https://developer.apple.com/documentation/bundleresources/information-property-list/itsappusesnonexemptencryption)).
- Apple's documentation table lists, for "an industry standard algorithm, not provided within the Apple operating system", only a **French encryption declaration**, and only when the app is distributed on the App Store **in France**. A CCATS is required only for proprietary or non-standard algorithms ([Export compliance documentation for encryption](https://developer.apple.com/help/app-store-connect/reference/app-information/export-compliance-documentation-for-encryption), [Overview of export compliance](https://developer.apple.com/help/app-store-connect/manage-app-information/overview-of-export-compliance)).
- On the US side, mass-market encryption items under EAR §740.17(b)(1) no longer need the annual self-classification report since the BIS rule of 29 March 2021 (86 FR 16482), and publicly available open-source encryption code using standard cryptography no longer needs the email notification ([Federal Register](https://www.govinfo.gov/content/pkg/FR-2021-03-29/pdf/2021-05481.pdf), [BIS: encryption items not subject to the EAR](https://www.bis.gov/learn-support/encryption-controls/encryption-items-not-subject-to-ear)).

Consequences for the owner:

- **France:** either exclude France from the app's availability, or file the French encryption declaration (ANSSI), upload it in App Store Connect → App Information → App Encryption Documentation, then set `ITSAppUsesNonExemptEncryption` to `true` and add the `ITSEncryptionExportComplianceCode` that Apple issues.
- If the encryption ever changes to a non-standard algorithm, or Clipsa starts sending encrypted data over the network (sync, for example), this decision has to be reviewed.
- Apple states that the developer carries the liability for a wrong exemption claim. This note is not legal advice.
