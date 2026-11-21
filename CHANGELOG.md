# Changelog

All notable changes to Clipsa are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and Clipsa uses
[Semantic Versioning](https://semver.org/).

Each release has a section headed `## [X.Y.Z] - YYYY-MM-DD` (`Unreleased` in
place of the date until it ships). That section is published as the GitHub
release notes, in Clipsa's update window and on
[clipsa.dev/changelog](https://clipsa.dev/changelog) (run `pnpm site`), so write
it for users; see [Release notes](./docs/deployment.md#release-notes) for the format.

## [0.1.5] - Unreleased

### Added

- Every release includes a `SHA256SUMS` file with the checksums of its installers.
- The download buttons on [clipsa.dev](https://clipsa.dev/#download) start the right installer for your system directly, then show how to install it and how to check the download.

### Changed

- The [privacy policy](https://clipsa.dev/privacy) now says "get" instead of "buy" for the Mac App Store edition, which will start out free. Nothing changes in what is collected: nothing.
- Settings only explains how things work on your own system: for example, excluded apps on a Mac ask for a bundle identifier, without the Windows and Linux notes.

## [0.1.4] - 2026-11-17

### Changed

- macOS: without the Accessibility permission, Enter in the quick picker now copies the item, closes the picker and takes you back to your app, so you can press ⌘V right away, instead of showing a message and staying open. The picker's footer shows "Copy then ⌘V" while pasting isn't possible.
- The website clipsa.dev moved to Cloudflare Pages. Its [privacy policy](https://clipsa.dev/privacy) now names Cloudflare as the host, which shows the project aggregate traffic statistics (requests, visitors, countries). The website still sets no cookies and runs no scripts, and the app still collects nothing.

## [0.1.3] - 2026-11-16

### Fixed

- macOS: Clipsa could quit unexpectedly when pasting from the quick picker, most likely after switching keyboard layouts or waking the Mac. The paste shortcut is now sent from the main thread, as macOS requires.

## [0.1.2] - 2026-11-16

### Added

- Settings… in the menu bar (tray) menu and, on macOS, in the Clipsa app menu with ⌘,.
- The tray menu shows the quick picker's shortcut and groups related items.

### Fixed

- macOS: with launch at login on, System Settings → Login Items and the "Background Items Added" notification now show Clipsa instead of the developer's name.

## [0.1.1] - 2026-11-15

### Changed

- Maintenance release with no changes to features. Releases are now built by a hardened pipeline: pinned build tools, signing keys available only to release builds, and published releases that can't be modified afterwards.

## [0.1.0] - 2026-11-15

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
