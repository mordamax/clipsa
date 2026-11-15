# Architecture

Clipsa is a single Tauri 2 process. The Rust backend does clipboard monitoring, encryption, storage and OS integration. A SvelteKit SPA (adapter-static, `ssr = false`) renders two webview windows. Nothing runs in a separate process or service.

```
┌──────────────────────── Tauri process ────────────────────────┐
│  Webview "main"  (/ , /settings)    Webview "picker" (/picker)│
│              │  invoke / events (snake_case JSON)  │          │
│  ┌───────────┴─────────────────────────────────────┴───────┐  │
│  │ lib.rs: commands, tray, windows, global hotkey, plugins │  │
│  ├──────────────┬──────────────┬──────────────┬────────────┤  │
│  │ clipboard.rs │  crypto.rs   │  storage.rs  │ settings.rs│  │
│  │ monitor loop │ AES-GCM, HMAC│ SQLite       │ validate,  │  │
│  │ capture/write│ keychain key │ retention    │ persist    │  │
│  ├──────────────┴──────────────┴──────────────┴────────────┤  │
│  │ platform.rs: change counter, sensitive-copy markers,    │  │
│  │ frontmost app, accessibility check, paste simulation    │  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
        arboard · keyring · rusqlite (bundled) · enigo
```

## Backend modules (`src-tauri/src`)

| Module | Responsibility |
|---|---|
| `lib.rs` | Builds the Tauri app: plugins (global-shortcut, autostart, single-instance, updater, process, dialog), tray menu (Open, Pause/Resume monitoring, Quit), close-to-tray, picker window show/hide, all `#[tauri::command]`s, and the hourly retention task. |
| `clipboard.rs` | Monitor loop. Reads the clipboard via `arboard` when the platform change signal fires, classifies the content (text, HTML + plain alt, PNG image, file list), applies the sensitive/excluded-app filters, and hands items to storage. Also writes stored items back to the clipboard in their native format. |
| `crypto.rs` | Loads or creates the 256-bit master key in the OS keychain (`keyring` 3), encrypts and decrypts with AES-256-GCM using random 96-bit nonces, and derives the HMAC-SHA256 dedup key with HKDF. Debug builds use a file-based dev key. |
| `storage.rs` | SQLite (`rusqlite`, bundled) with `secure_delete=ON`: insert/dedup/bump, paging and search, statistics, deletion, and pruning by `max_items` and `auto_cleanup_days`. |
| `platform.rs` | OS specifics: NSPasteboard `changeCount` / `GetClipboardSequenceNumber` / polling + hash on Linux; password-manager markers; frontmost-app id; macOS Accessibility status; Cmd/Ctrl+V simulation. |
| `settings.rs` | `AppSettings` model, defaults, validation, persistence, and applying changes (hotkey re-registration, autostart, monitoring toggle). |
| `error.rs` | Error type. Commands reject with a plain string. |
| `updater.rs` | Direct-download builds only (`self-update` feature): creates the hidden "Software Update" window, shows it (`show_updater`, tray "Check for Updates…") and saves its choices (`update_updater_settings`, allowed while locked). |

## Data flow

1. **Capture.** The platform change signal fires and the frontmost app is checked against `excluded_apps`. If `ignore_sensitive` is on, concealed/transient copies are skipped. Otherwise the content is read and its HMAC computed. A duplicate only bumps `last_used_at`. New content is encrypted and inserted, retention runs, and `clipboard://changed` is emitted.
2. **Browse.** The UI calls `get_history`. Rows are decrypted to build previews and image thumbnails. Search is a case-insensitive substring match over the decrypted text.
3. **Re-use.** `copy_item` writes the item to the clipboard and bumps it. `paste_item` also hides the picker and simulates Cmd/Ctrl+V into the previously focused app.

Our own write-backs are not re-captured as new items; they dedup to the existing row.

## Frontend (`src`)

- `routes/+page.svelte`: history list with search, type filter, copy/delete, clear, statistics.
- `routes/settings`: all `AppSettings`, Accessibility permission status, update preferences.
- `routes/picker`: compact keyboard-driven list (type to filter, ↑/↓, Enter pastes, Esc hides). It reloads on `picker://shown`.
- `routes/updater`: the hidden "Software Update" window (direct-download builds). It is the only place that checks for updates: at startup and once a day if `check_for_updates` is on, and on `updater://check` (`{ manual }`). Automatic checks respect "Skip This Version" (`skipped_update_version`) and "Remind Me Later" (`update_snoozed_until`), and with `auto_install_updates` install silently for the next launch; otherwise the window appears with the release notes from `latest.json`.

The command/event contract (names, args, payloads) is documented in the backend command handlers in `lib.rs`. All fields are snake_case.

## Storage layout

- Release: `<data dir>/clipsa/clipboard.db`.
- Debug: `<data dir>/clipsa-dev/`, with a file-based dev key instead of the keychain.

Each item row stores the ciphertext and nonce plus plaintext metadata: id, content type, size, created/last-used timestamps, and the HMAC dedup tag.
