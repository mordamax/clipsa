# Roadmap

## Shipped

- Clipboard capture of text, rich text (HTML + plain-text alternative), images (PNG) and file lists on macOS, Windows and Linux
- Native change detection (macOS `changeCount`, Windows sequence number); polling on Linux
- Per-item AES-256-GCM encryption, master key in the OS keychain, HMAC-SHA256 dedup, `secure_delete`
- Refusal to start on keychain failure (history is never orphaned); separate dev data dir and key
- Skipping password-manager copies (macOS, Windows) and per-app exclusions (macOS, Windows)
- Retention by item count and age, enforced on capture and hourly
- History window with search, type filter, copy, delete, clear and statistics
- Quick picker on a configurable global hotkey, with keyboard navigation and paste into the previous app
- Tray menu, close-to-tray, pause/resume monitoring, single instance, launch at login
- Settings: limits, theme, compact mode, timestamps, page size, hotkey, exclusions
- Signed auto-updates via GitHub Releases; CI on all three OSes; release workflow with macOS signing and notarization
- Pinned items and snippets (exempt from retention), paste as plain text, app lock (Touch ID / Windows Hello)
- Sparkle-style update window with release notes; Mac App Store edition; Homebrew cask

## Next

Ordered by what users of other clipboard managers value most and Clipsa lacks (see the
[comparison](https://clipsa.dev/compare), researched November 2026):

1. **Text recognition in images (OCR):** search and copy text in screenshots. Maccy, Paste, ClipBook and Raycast have it; on macOS via Apple Vision (on-device), Windows via `Windows.Media.Ocr`.
2. **Paste stack:** select several items and paste them in order (Paste, ClipBook, Raycast, Pastebot).
3. **Text actions:** common transformations from the picker (case, trim, JSON pretty/minify, URL/Base64 encode, line operations).
4. **Encrypted sync** between devices: opt-in and end-to-end encrypted, the gap Paste and Pastebot fill with iCloud. Needs a design that keeps "no server sees plaintext".
5. **Windows code signing** (SmartScreen warnings).
6. **Export / import** of encrypted history (moving to a new computer).
7. **Linux sensitive-copy detection** (`x-kde-passwordManagerHint`) and frontmost-app exclusion on X11.
8. **Encrypted metadata:** encrypt timestamps, type and size, or move to a fully encrypted database.
9. **Full-text index** for faster search on very large histories.
