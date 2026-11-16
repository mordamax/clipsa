#!/usr/bin/env bash
# Writes the in-app updater's latest.json for a release to stdout, from the
# updater bundles' .sig files (as uploaded by the release builds).
#
#   scripts/updater-manifest.sh <version> <dir with .sig files> <notes file> <download base URL>
#
# Every platform is required: a missing .sig fails with a list, so a release
# can't be published with an update file that leaves a platform behind. The
# release workflow generates this once, after all builds, instead of letting
# each parallel build job merge its entry into a shared file.
set -euo pipefail

version="$1" sigs="$2" notes="$3" base="$4"

# updater platform key → bundle file name (keys as tauri-plugin-updater expects)
manifest=(
  "darwin-aarch64         Clipsa_${version}_aarch64.app.tar.gz"
  "darwin-aarch64-app     Clipsa_${version}_aarch64.app.tar.gz"
  "darwin-x86_64          Clipsa_${version}_x64.app.tar.gz"
  "darwin-x86_64-app      Clipsa_${version}_x64.app.tar.gz"
  "linux-x86_64           Clipsa_${version}_amd64.AppImage"
  "linux-x86_64-appimage  Clipsa_${version}_amd64.AppImage"
  "linux-x86_64-deb       Clipsa_${version}_amd64.deb"
  "linux-x86_64-rpm       Clipsa-${version}-1.x86_64.rpm"
  "windows-x86_64         Clipsa_${version}_x64_en-US.msi"
  "windows-x86_64-msi     Clipsa_${version}_x64_en-US.msi"
  "windows-x86_64-nsis    Clipsa_${version}_x64-setup.exe"
)

missing=()
platforms='{}'
for entry in "${manifest[@]}"; do
  read -r key file <<<"$entry"
  if [ ! -s "$sigs/$file.sig" ]; then
    missing+=("$file.sig")
    continue
  fi
  platforms="$(jq --arg key "$key" --arg url "$base$(jq -rn --arg f "$file" '$f | @uri')" \
    --rawfile sig "$sigs/$file.sig" '.[$key] = {signature: $sig, url: $url}' <<<"$platforms")"
done
if [ ${#missing[@]} -gt 0 ]; then
  printf 'Missing updater signatures: %s\n' "$(printf '%s ' "${missing[@]}" | xargs -n1 | sort -u | xargs)" >&2
  exit 1
fi

jq -n --arg version "$version" --rawfile notes "$notes" --argjson platforms "$platforms" \
  --arg pub_date "$(date -u +%Y-%m-%dT%H:%M:%S.000Z)" \
  '{version: $version, notes: ($notes | sub("\n+$"; "")), pub_date: $pub_date, platforms: $platforms}'
