# Architecture

Clipsa is a single Tauri 2 app. The Rust backend does clipboard monitoring, encryption, storage and OS integration. A SvelteKit SPA (adapter-static, `ssr = false`) renders the webview windows: `main` (`/`, `/settings`), `picker` (`/picker`) and, in direct-download builds, `updater` (`/updater`). There is no separate daemon or service.

```
┌──────────────────────── Tauri process ────────────────────────┐
│  Webview "main"  (/ , /settings)    Webview "picker" (/picker)│
│  Webview "updater" (/updater, direct-download builds only)    │
│              │  invoke / events (snake_case JSON)  │          │
│  ┌───────────┴─────────────────────────────────────┴───────┐  │
│  │ lib.rs: commands, tray, windows, global hotkey, plugins │  │
│  ├──────────────┬──────────────┬──────────────┬────────────┤  │
│  │ clipboard.rs │  crypto.rs   │  storage.rs  │ settings.rs│  │
│  │ monitor loop │ AES-GCM, HMAC│ SQLite       │ validate,  │  │
│  │ capture/write│ keychain key │              │ persist    │  │
│  │ retention    │              │              │            │  │
│  ├──────────────┴──────────────┴──────────────┴────────────┤  │
│  │ platform.rs: change counter, sensitive-copy markers,    │  │
│  │ frontmost app, accessibility, OS auth, paste simulation │  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
        arboard · keyring · rusqlite (bundled) · enigo
```

## Backend modules (`src-tauri/src`)

| Module | Responsibility |
|---|---|
| `lib.rs` | Builds the Tauri app: plugins (single-instance, dialog, process, opener, global-shortcut; updater with the `self-update` feature), the launch-at-login entry, tray menu (Open Clipsa, Quick picker, Pause/Resume monitoring, Settings…, Check for Updates…, Quit), the macOS app menu (Tauri's default plus Settings… ⌘,), close-to-tray, picker window show/hide, and all `#[tauri::command]`s. |
| `clipboard.rs` | Monitor loop. Reads the clipboard via `arboard` when the platform change signal fires, classifies the content (text, HTML + plain alt, PNG image, file list), applies the sensitive/excluded-app filters, and hands items to storage. Writes stored items back to the clipboard in their native format, and runs retention after each capture and on an hourly background thread. |
| `crypto.rs` | Loads or creates the 256-bit master key (`keyring` 3 for the direct download; the macOS data protection keychain for the App Store edition), encrypts and decrypts with AES-256-GCM using random 96-bit nonces, and derives the HMAC-SHA256 dedup key with HKDF. Debug builds use a file-based dev key. The key is loaded at startup only once the first-run screen has been completed; before that, that screen creates it (`set_up_encryption`), and capture and the quick picker start then. |
| `storage.rs` | SQLite (`rusqlite`, bundled) with `secure_delete=ON`: schema migrations, insert/dedup/bump, paging and search, pins and titles, statistics, deletion, and pruning by `max_items` and `auto_cleanup_days`. |
| `platform.rs` | OS specifics: NSPasteboard `changeCount` / `GetClipboardSequenceNumber` / polling + hash on Linux; password-manager markers; frontmost-app id; macOS Accessibility status; Touch ID / Windows Hello authentication; Cmd/Ctrl+V simulation. |
| `lock.rs` | App lock state: locks after the configured time in the background, unlocks through `platform.rs` authentication, and rejects history commands while locked. |
| `settings.rs` | `AppSettings` model, defaults, validation, persistence, and applying changes (hotkey re-registration, autostart, monitoring toggle). |
| `error.rs` | Error type. Commands reject with a plain string. |
| `updater.rs` | Direct-download builds only (`self-update` feature): creates the hidden "Software Update" window, shows it (`show_updater`, tray "Check for Updates…") and saves its choices (`update_updater_settings`, allowed while locked). |

## Data flow

1. **Capture.** The platform change signal fires. If `ignore_sensitive` is on, concealed/transient copies are skipped, and copies made while an app in `excluded_apps` is frontmost are skipped. Otherwise the content is read and its HMAC computed. A duplicate only bumps `last_used_at`. New content is encrypted and inserted, retention runs, and `clipboard://changed` is emitted.
2. **Browse.** The UI calls `get_history`. Each row's encrypted preview (up to 300 characters of text, or an image thumbnail) and title are decrypted; full content is not loaded. Search is a case-insensitive substring match over the decrypted text and titles; images match by title only.
3. **Re-use.** `copy_item` writes the item to the clipboard and bumps it. `paste_item` also hides the picker, returns focus to the previously focused app and simulates Cmd/Ctrl+V there; without macOS Accessibility permission it stops after the copy and reports `copied` instead of `pasted`.

Clipsa's own write-backs are not re-captured as new items; they dedup to the existing row.

## Frontend (`src`)

- `routes/+page.svelte`: history list with search, type filter, copy/delete, clear, statistics.
- `routes/settings`: all `AppSettings`, Accessibility permission status, update preferences.
- `routes/picker`: compact keyboard-driven list (type to filter, ↑/↓, Enter pastes, Esc hides). It reloads on `picker://shown`.
- `routes/updater`: the hidden "Software Update" window (direct-download builds). It is the only place that checks for updates: at startup and once a day if `check_for_updates` is on, and on `updater://check` (`{ manual }`). Automatic checks respect "Skip This Version" (`skipped_update_version`) and "Remind Me Later" (`update_snoozed_until`), and with `auto_install_updates` install silently for the next launch; otherwise the window appears with the release notes from `latest.json`.

The command/event contract (names, args, payloads) is documented in the backend command handlers in `lib.rs`. All fields are snake_case.

## Storage layout

- Release: `<data dir>/clipsa/clipboard.db`.
- Debug: `<data dir>/clipsa-dev/`, with a file-based dev key instead of the keychain.

`<data dir>` is `~/Library/Application Support` on macOS, `%APPDATA%` on Windows and `$XDG_DATA_HOME` (or `~/.local/share`) on Linux. The Mac App Store edition keeps it inside its sandbox container (`~/Library/Containers/com.clipsa.app/Data/Library/Application Support`).

Each item row stores the encrypted content, preview and optional title (each with its nonce) plus plaintext metadata: id, content type, size, created/last-used timestamps, the pinned flag, and the HMAC dedup tag.

## Repository layout

```
clipsa/
├── src/                          # SvelteKit SPA (see Frontend above)
├── src-tauri/
│   ├── src/                      # Rust backend (see Backend modules above)
│   ├── capabilities/             # Tauri permission capabilities
│   ├── appstore/                 # Mac App Store Info.plist and entitlements template
│   ├── tauri.conf.json           # App config (the version comes from Cargo.toml)
│   ├── tauri.release.conf.json   # Release overlay: enables updater artifacts
│   └── tauri.appstore.conf.json  # Mac App Store overlay
├── scripts/                      # Site generator, release helpers, local macOS signing and install
├── packaging/homebrew/           # Homebrew cask template
├── site/                         # clipsa.dev static website
├── docs/
├── .changes/                     # Release notes waiting for the next release (covector)
├── CHANGELOG.md                  # Release notes (published with each release)
├── AGENTS.md, CLAUDE.md          # Instructions for AI coding agents (CLAUDE.md imports AGENTS.md)
└── .github/workflows/            # ci.yml, release-pr.yml, release.yml, appstore.yml, site*.yml
```
