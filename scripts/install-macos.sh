#!/usr/bin/env bash
# Build a release Clipsa.app and install it into /Applications (macOS only).
#
# Signing: uses $CLIPSA_SIGNING_IDENTITY if set, else the "Clipsa Local Signing"
# certificate if it is in the keychain, else an ad-hoc signature. With an
# ad-hoc signature macOS treats every rebuild as a new app, so Accessibility
# and "Always Allow" keychain access must be granted again after each install;
# a stable identity keeps them. Create the local identity once with
# scripts/create-local-signing-identity.sh.
set -euo pipefail

[[ "$(uname)" == "Darwin" ]] || { echo "install-macos.sh only runs on macOS" >&2; exit 1; }
cd "$(dirname "$0")/.."

pnpm tauri build --bundles app

app="src-tauri/target/release/bundle/macos/Clipsa.app"
if [[ -n "${CLIPSA_SIGNING_IDENTITY:-}" ]]; then
  identity="$CLIPSA_SIGNING_IDENTITY"
elif security find-certificate -c "Clipsa Local Signing" >/dev/null 2>&1; then
  identity="Clipsa Local Signing"
else
  identity="-"
fi
codesign --force --deep --sign "$identity" "$app"
codesign --verify --deep --strict "$app"

# Stop any running Clipsa release build (no Apple Events, so no Automation prompt).
pkill -f "Clipsa.app/Contents/MacOS/clipsa" || true
sleep 1

rm -rf /Applications/Clipsa.app
ditto "$app" /Applications/Clipsa.app
# Remove the build output so Launchpad/Spotlight only ever list the installed copy.
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -u "$app" || true
rm -rf "$app"
open /Applications/Clipsa.app
echo "Installed /Applications/Clipsa.app (signed with: ${identity/#-/ad-hoc})"
