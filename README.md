# Clipsa

A local-first, encrypted clipboard history manager for macOS, Windows and Linux, built with Rust, Tauri 2 and SvelteKit.

Clipsa records what you copy, encrypts every item on disk, and lets you find and re-paste it from a keyboard-driven quick picker. It makes no network requests unless you opt in to the update check on the first-run screen, and it has no telemetry.

Website: **[clipsa.dev](https://clipsa.dev)** · How it compares with Maccy, Paste, Raycast and others: **[clipsa.dev/compare](https://clipsa.dev/compare)**

## Install

- **macOS** (Apple Silicon and Intel, signed and notarized): download the `.dmg` from [Releases](https://github.com/mordamax/clipsa/releases/latest), or `brew install --cask mordamax/tap/clipsa`.
- **Windows:** the `.msi` or `-setup.exe` from [Releases](https://github.com/mordamax/clipsa/releases/latest). The installers are not code-signed yet, so SmartScreen may warn.
- **Linux:** the `.AppImage`, `.deb` or `.rpm` from [Releases](https://github.com/mordamax/clipsa/releases/latest). Needs a Secret Service provider (GNOME Keyring, KWallet) to store the key.

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
- **Tray icon** with Open Clipsa, Quick picker (with its shortcut), Pause/Resume monitoring, Settings…, Check for Updates… (direct downloads) and Quit. Closing the window hides Clipsa to the tray. On macOS, Settings… is also in the Clipsa menu (⌘,).
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

- Each item's content is encrypted with AES-256-GCM under a random 256-bit master key kept in the OS keychain. If keychain access fails, Clipsa refuses to start rather than create a new key and orphan your history.
- Duplicates are detected with a keyed HMAC-SHA256, so the database holds no plain hash of your content. Deleted rows are overwritten (`secure_delete`).
- **Not encrypted:** metadata such as content type, size, timestamps and the pinned flag, and settings.
- History is stored in `<data dir>/clipsa/clipboard.db` ([locations per OS](./docs/architecture.md#storage-layout)).

Clipsa protects history at rest, for example a copied or backed-up database file. It is not a sandbox against malware running as your user, which can read the clipboard directly. See [docs/security.md](./docs/security.md) for the full model and its limits.

## Development

Install Rust stable, Node.js 22, pnpm 9 and the Tauri 2 system dependencies ([details](./docs/contributing.md#setup)), then:

```bash
pnpm install
pnpm tauri dev      # development build: separate data directory and key, no keychain prompts
pnpm tauri build    # installers in src-tauri/target/release/bundle/
pnpm install:mac    # macOS: build a release Clipsa.app, sign it and install it into /Applications
```

- Checks, tests and code guidelines: [docs/contributing.md](./docs/contributing.md)
- Local signing for `install:mac` and updater artifacts: [docs/deployment.md](./docs/deployment.md#local-builds)
- Cutting a release (changelog, version bump, tag, workflow secrets): [docs/deployment.md](./docs/deployment.md#cutting-a-release)

## Documentation

- [Architecture](./docs/architecture.md)
- [Security model](./docs/security.md)
- [Building and releasing](./docs/deployment.md)
- [Contributing](./docs/contributing.md)
- [Roadmap](./docs/development-roadmap.md)

## Reporting security issues

Please report vulnerabilities privately through [GitHub security advisories](https://github.com/mordamax/clipsa/security/advisories/new) rather than public issues.

## License

MIT. See [LICENSE](./LICENSE).
