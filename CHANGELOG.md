# Changelog

All notable changes to Clipsa are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and Clipsa uses
[Semantic Versioning](https://semver.org/).

Each release has a section headed `## [X.Y.Z] - YYYY-MM-DD` (or
`## [X.Y.Z] - Unreleased` until it ships). The release workflow publishes the
section for the version being released as the GitHub release notes and as the
"Release Notes" shown in Clipsa's update window, so write it for users.

## [0.1.0] - Unreleased

First public release.

### Added

- Clipboard history for text, rich text, images and copied files. Copying an item again puts it back on the clipboard in its original format.
- Every item is encrypted on your device with AES-256-GCM, using a key kept in your system keychain (macOS Keychain, Windows Credential Manager or Linux Secret Service).
- Quick picker on a global shortcut (`Cmd/Ctrl+Shift+V` by default): type to filter, press Enter to paste straight into the app you were using, `Shift+Enter` to paste the opposite way (plain text or formatted), or `Cmd/Ctrl+1`–`9` to paste one of the first nine items. Items are grouped by day, `Tab` switches between type filters, and a preview pane shows the selected item in full with its details. Buttons in the picker, or `Cmd/Ctrl+O` and `Cmd/Ctrl+,`, open the full Clipsa window and Settings.
- A history window with search, type filters, a list grouped by day and a detail pane for the selected item. Clicking an item copies it.
- Pin items to keep them forever, give them names, and write reusable text snippets. Pinned items appear first in the quick picker.
- Paste as plain text by default if you prefer.
- App lock on macOS and Windows: require Touch ID, your login password or Windows Hello to see your history after Clipsa has been in the background for a while.
- Copies that password managers mark as secret are never saved, and you can exclude whole apps.
- Retention limits: keep at most a chosen number of items and delete items older than a chosen number of days.
- Tray icon to open Clipsa, pause or resume recording, and quit. Closing the window keeps Clipsa running in the tray.
- Launch at login, light/dark/system theme and a compact layout.
- Signed automatic updates for downloads from GitHub and Homebrew, only if you opt in. A "Software Update" window shows what's new and lets you install now, skip a version or be reminded later, or have updates installed automatically.
- A Mac App Store edition. It runs in Apple's sandbox, so Enter in the quick picker copies the item for you to paste with ⌘V, updates come from the App Store, and launch at login uses macOS Login Items.
