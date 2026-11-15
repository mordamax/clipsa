#!/usr/bin/env bash
# Create a self-signed "Clipsa Local Signing" code-signing identity in the login
# keychain (macOS, maintainers only). scripts/install-macos.sh signs local builds
# with it, so macOS sees every rebuild as the same app: keychain "Always Allow"
# and Accessibility access survive reinstalls. Release builds are signed with the
# Developer ID in CI and do not use this.
#
# Remove it again with:
#   security delete-identity -c "Clipsa Local Signing" ~/Library/Keychains/login.keychain-db
set -euo pipefail

name="Clipsa Local Signing"
if security find-certificate -c "$name" >/dev/null 2>&1; then
  echo "\"$name\" already exists."
  exit 0
fi

dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT
cd "$dir"

/usr/bin/openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 3650 -nodes \
  -subj "/CN=$name" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=critical,codeSigning" \
  -addext "basicConstraints=critical,CA:false" 2>/dev/null
/usr/bin/openssl pkcs12 -export -inkey key.pem -in cert.pem -out identity.p12 -passout pass:clipsa-local 2>/dev/null
# -T: let codesign use the key without a keychain prompt.
security import identity.p12 -k ~/Library/Keychains/login.keychain-db -P clipsa-local -T /usr/bin/codesign >/dev/null

echo "Created \"$name\". scripts/install-macos.sh will now sign with it."
