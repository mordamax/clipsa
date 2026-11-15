# Clipsa

A local-first, encrypted clipboard history manager for macOS, Windows and Linux, built with Rust, Tauri 2 and SvelteKit.

Clipsa records what you copy, encrypts every item on disk, and lets you find and re-paste it from a keyboard-driven quick picker. It makes no network requests unless you opt in to the update check on the first-run screen, and it has no telemetry.

Website: **[clipsa.dev](https://clipsa.dev)** · How it compares with Maccy, Paste, Raycast and others: **[clipsa.dev/compare](https://clipsa.dev/compare)**

## Features

- **Captures text, rich text, images and file lists.** Rich text keeps both the HTML and its plain-text alternative; images are stored as PNG; file copies are stored as their path list. Re-copying an item puts it back on the clipboard in its native format.
- **Per-item encryption.** Each item is encrypted individually with AES-256-GCM before it touches SQLite.
- **Master key in the OS keychain** (macOS Keychain, Windows Credential Manager, Linux Secret Service).
- **Quick picker** opened by a global hotkey (default `Cmd/Ctrl+Shift+V`). Press Enter to paste the selected item straight into the app you were using.
- **Search and filter** history by content type and by substring over the decrypted text and titles.
- **Pinned items and snippets.** Pin anything to keep it forever (pinned items are exempt from retention and "Clear history"), give it a name, or write reusable text snippets. Pinned items appear first in the quick picker.
- **Paste as plain text** with `Shift+Enter` in the picker, or make it the default in Settings.
- **App lock** (macOS, Windows): require Touch ID / your login password (macOS) or Windows Hello to see history after Clipsa has been in the background for a chosen time. Capture keeps running while locked.
- **Skips password-manager copies** that are marked concealed or transient (macOS and Windows).
- **Per-app exclusions:** never record copies made while a given app is in front (macOS bundle id, e.g. `com.agilebits.onepassword7`; Windows exe name, e.g. `keepass.exe`).
- **Retention limits:** maximum item count and automatic deletion after N days. Both are enforced after every capture and once an hour.
- **Tray icon** with Open, Pause/Resume monitoring, Check for Updates… (direct downloads) and Quit. Closing the window hides Clipsa to the tray.
- Single instance, optional launch at login, light/dark/system theme, compact mode.
- **Signed auto-updates** from GitHub Releases (direct downloads and Homebrew), only if you opt in on the first-run screen or in Settings. A "Software Update" window shows the release notes and offers Install Update, Skip This Version and Remind Me Later; optionally, updates install automatically and take effect at the next launch.

## Keyboard usage

| Where | Key | Action |
|---|---|---|
| Anywhere | `Cmd/Ctrl+Shift+V` (configurable; empty disables) | Open the quick picker |
| Picker | type | Filter history |
| Picker | `↑` / `↓` | Move the selection |
| Picker | `Enter` | Paste the selected item into the previously focused app |
| Picker | `Shift+Enter` | Paste the opposite way: plain text if the default keeps formatting, and vice versa |
| Picker | `Cmd/Ctrl+1` … `Cmd/Ctrl+9` | Paste the 1st … 9th item in the list (add `Shift` for the opposite format) |
| Picker | `Tab` / `Shift+Tab` | Next / previous type filter (All, Text, Rich, Images, Files, Pinned) |
| Picker | `Cmd/Ctrl+C` or click | Copy the item without pasting |
| Picker | `Esc` | Close the picker |
| Picker | `Cmd/Ctrl+O` | Open the full Clipsa window (also the window button in the picker's header) |
| Picker | `Cmd/Ctrl+,` | Open Settings (also the gear button in the picker's header) |
| History window | click | Copy the item and show it in the detail pane |
| History window | `↑` / `↓` | Move through the list |
| History window | `Cmd/Ctrl+F` | Focus the search field |

On macOS, pasting into another app requires the **Accessibility** permission (System Settings → Privacy & Security → Accessibility). Clipsa can request it from Settings. Until it is granted, use the history window to copy an item and paste it yourself.

### Mac App Store edition

The App Store build is the same app with these differences, all required by the App Store sandbox:

- In the picker, `Enter` (and `Cmd/Ctrl+1` … `9`) **copies** the item and closes the picker; press `⌘V` yourself. The sandbox does not allow sending keystrokes to other apps, so no Accessibility permission is involved.
- Updates come from the App Store; there is no built-in updater.
- Launch at login uses macOS Login Items (System Settings → General → Login Items). While it is on, Clipsa always starts in the menu bar.
- History lives in the app's sandbox container (`~/Library/Containers/com.clipsa.app/…`). It is separate from the direct-download edition and is not migrated.

See [docs/deployment.md](./docs/deployment.md#mac-app-store) for build and submission details.

## Platform support

| Capability | macOS | Windows | Linux |
|---|---|---|---|
| Text / rich text / images / files capture | ✅ | ✅ | ✅ |
| Change detection | NSPasteboard `changeCount` | `GetClipboardSequenceNumber` | Polling + content hash |
| Skip password-manager copies | ✅ (`org.nspasteboard.ConcealedType` / `TransientType` / `AutoGeneratedType`) | ✅ (`ExcludeClipboardContentFromMonitorProcessing`, `Clipboard Viewer Ignore`) | ❌ not available |
| Per-app exclusion | ✅ bundle id | ✅ exe name | ❌ not available |
| Paste into previous app | ✅ (needs Accessibility) | ✅ | ✅ on X11; Wayland compositors may block simulated keystrokes |
| App lock | ✅ Touch ID / login password | ✅ Windows Hello (must be set up) | ❌ not available |
| Master key storage | Keychain | Credential Manager | Secret Service (GNOME Keyring, KWallet, …) |
| Release artifacts | `.dmg` / `.app` (Apple Silicon and Intel) | `.msi`, NSIS `.exe` | `.AppImage`, `.deb`, `.rpm` |

On Linux, Clipsa cannot tell which app owns a copy and cannot see password-manager markers. Use **Pause monitoring** in the tray, or delete the item afterwards.

## Security model

What is actually implemented:

- **Content encryption.** Each clipboard item's content (text, HTML, image bytes, file paths) is encrypted with AES-256-GCM using a fresh random 96-bit nonce.
- **Master key.** A random 256-bit key is generated on first run and stored in the OS keychain. If keychain access is denied or fails, Clipsa shows an error and refuses to start. It never silently creates a new key, because a new key would make existing history unreadable.
- **Deduplication without plaintext hashes.** Duplicates are detected with HMAC-SHA256. Its key is derived from the master key with HKDF, so the database holds no plain hash of your content.
- **Deletion.** SQLite runs with `secure_delete=ON`, so deleted rows are overwritten in the database file.
- **Not encrypted:** metadata such as content type, size, creation and last-used timestamps, and settings. The database file is plain SQLite (no SQLCipher). Only item contents are ciphertext.
- **Data location:** `<data dir>/clipsa/clipboard.db`. The data dir is `~/Library/Application Support` on macOS, `%APPDATA%` on Windows and `$XDG_DATA_HOME` / `~/.local/share` on Linux.
- **Network:** none by default. The first-run screen asks whether Clipsa may check GitHub Releases for updates at startup and once a day (changeable in Settings). Updates are verified against the minisign public key built into the app.

Limits: anyone who can run code as your user while you are logged in can ask the keychain for the key (subject to OS prompts) or read the clipboard directly. Clipsa protects history at rest, for example a copied or backed-up database file. It is not a sandbox against local malware.

See [docs/security.md](./docs/security.md) for details.

## Development

### Prerequisites

- Rust stable (`rustup`)
- Node.js 22 and pnpm 9
- Tauri 2 system dependencies for your OS ([guide](https://v2.tauri.app/start/prerequisites/)). On Debian/Ubuntu:
  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev \
    patchelf libxdo-dev libssl-dev libdbus-1-dev
  ```

### Run

```bash
pnpm install
pnpm tauri dev
```

Debug builds keep their data in a separate directory, `<data dir>/clipsa-dev/`, and use a file-based development key instead of the keychain. Rebuilding during development therefore never triggers keychain prompts and never touches your real history.

### Checks

```bash
pnpm check                      # Svelte / TypeScript
pnpm build                      # builds the SPA into build/ (needed before cargo commands on a clean checkout)
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test                      # uses injected keys; does not touch the OS keychain
```

### Build

```bash
pnpm tauri build
```

Installers are written to `src-tauri/target/release/bundle/`.

To also produce signed updater artifacts (`*.app.tar.gz`, `*.sig`, …), build with the release config overlay and the updater signing key:

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/clipsa-updater.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""   # if the key has a password
pnpm tauri build --config src-tauri/tauri.release.conf.json
```

To build a release `Clipsa.app` and install it into `/Applications` on macOS:

```bash
scripts/create-local-signing-identity.sh   # once: self-signed "Clipsa Local Signing" in your login keychain
pnpm install:mac                           # build, sign, install into /Applications and launch
```

The installed app is a release build: it uses the real data directory and the keychain-stored key. `install:mac` signs with the local identity when it exists (or `$CLIPSA_SIGNING_IDENTITY`), so macOS sees each rebuild as the same app and keychain "Always Allow" and Accessibility access are kept. Without it the build is ad-hoc signed, and macOS asks for keychain access and Accessibility again after every install.

## Releasing

1. Add the release notes to [`CHANGELOG.md`](./CHANGELOG.md) under `## [X.Y.Z] - YYYY-MM-DD` (Keep a Changelog style; `## [X.Y.Z] - Unreleased` while in progress). Write for users: this section becomes the GitHub release text and the "Release Notes" in Clipsa's update window. The release fails before building if the section for the version is missing or empty.
2. Bump `version` in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json`, then commit.
3. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`. The tag must match the version (`v` + version).
4. The [Release workflow](./.github/workflows/release.yml) builds macOS (arm64 and x64), Linux and Windows, uploads the installers, updater bundles and `latest.json` (with the changelog section as `notes`) to a draft release, and publishes it once every build has succeeded. Installed apps then pick up the update from `https://github.com/mordamax/clipsa.dev/releases/latest/download/latest.json`.

You can also start the workflow by hand from the Actions tab (`workflow_dispatch`). It uses the version in `tauri.conf.json`. See [docs/deployment.md](./docs/deployment.md#release-notes) for the release notes format.

### Required GitHub secrets

| Secret | Required | Purpose |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | **yes** | Contents of `~/.tauri/clipsa-updater.key` on the maintainer machine. Signs the updater artifacts. Without it the release build fails. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | if the key has one | Password for the updater key |
| `APPLE_CERTIFICATE` | yes | Single-line base64 of the exported Developer ID Application `.p12` (`openssl base64 -A -in cert.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | yes | Password of the `.p12` |
| `APPLE_SIGNING_IDENTITY` | optional | e.g. `Developer ID Application: Name (TEAMID)`. Taken from the certificate if omitted. |
| `APPLE_ID` | yes | Apple ID email (notarization) |
| `APPLE_PASSWORD` | yes | App-specific password for that Apple ID |
| `APPLE_TEAM_ID` | yes | Apple Developer Team ID |
| `HOMEBREW_TAP_TOKEN` | optional | Fine-grained token with contents:write on `mordamax/homebrew-tap`. After publishing, the workflow renders the Homebrew cask from `packaging/homebrew/clipsa.rb.in`, attaches it to the release and, with this token, pushes it to the tap (`brew install --cask mordamax/tap/clipsa`). |

`GITHUB_TOKEN` is provided automatically. The release fails if any required Apple secret is missing: unsigned or un-notarized Mac builds trigger Gatekeeper warnings and, because each update would look like a different app, repeated keychain prompts and lost Accessibility access for users. Windows builds are not code-signed yet (SmartScreen warns until the app has reputation or is signed).

**Keep `~/.tauri/clipsa-updater.key` backed up.** Its public half is built into every installed copy. If the private key is lost, existing installs can never auto-update again.

## Project structure

```
clipsa/
├── src/                          # SvelteKit SPA (adapter-static, ssr = false)
│   └── routes/
│       ├── +page.svelte          # History window (main)
│       ├── settings/             # Settings page
│       ├── picker/               # Quick picker window
│       └── updater/              # "Software Update" window (direct downloads)
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                # App setup, commands, tray, windows, hotkey
│   │   ├── clipboard.rs          # Clipboard monitoring, capture, write-back
│   │   ├── crypto.rs             # AES-256-GCM, HKDF/HMAC, keychain master key
│   │   ├── storage.rs            # SQLite history store, retention
│   │   ├── platform.rs           # Change counters, sensitive-copy and frontmost-app detection, paste
│   │   ├── settings.rs           # Settings model, validation, persistence
│   │   ├── error.rs              # Error type
│   │   ├── updater.rs            # Update window plumbing (self-update feature)
│   │   └── main.rs
│   ├── capabilities/             # Tauri permission capabilities
│   ├── tauri.conf.json
│   └── tauri.release.conf.json   # Release overlay: enables updater artifacts
├── docs/
├── CHANGELOG.md                  # Release notes (published with each release)
└── .github/workflows/            # ci.yml, release.yml
```

## Documentation

- [Architecture](./docs/architecture.md)
- [Security model](./docs/security.md)
- [Building and releasing](./docs/deployment.md)
- [Contributing](./docs/contributing.md)
- [Roadmap](./docs/development-roadmap.md)

## Reporting security issues

Please report vulnerabilities privately through [GitHub security advisories](https://github.com/mordamax/clipsa.dev/security/advisories/new) rather than public issues.

## License

MIT. See [LICENSE](./LICENSE).
