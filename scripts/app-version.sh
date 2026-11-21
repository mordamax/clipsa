#!/usr/bin/env bash
# Prints Clipsa's version: the [package] version in src-tauri/Cargo.toml, the
# only place it is set (tauri.conf.json has none, so Tauri uses this one too).
set -euo pipefail
cd "$(dirname "$0")/.."
version="$(awk '/^\[/ { section = $0 } section == "[package]" && /^version = "/ { gsub(/^version = "|"$/, ""); print; exit }' src-tauri/Cargo.toml)"
[ -n "$version" ] || { echo "src-tauri/Cargo.toml has no [package] version" >&2; exit 1; }
printf '%s\n' "$version"
