# Contributing to Clipsa

## Setup

Requirements: Rust stable (`rustup`), Node.js 22, pnpm 9 and the [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/). On Debian/Ubuntu, CI installs:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev \
  patchelf libxdo-dev libssl-dev libdbus-1-dev libgtk-3-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

```bash
git clone https://github.com/mordamax/clipsa.git
cd clipsa
pnpm install
pnpm tauri dev
```

Debug builds keep their data in a separate directory, `<data dir>/clipsa-dev/`, and use a file-based key instead of the keychain, so development never triggers keychain prompts and never touches your real history.

## Before opening a PR

Run the same checks as CI:

```bash
pnpm check                      # Svelte / TypeScript
pnpm build                      # builds the SPA into build/ (needed before cargo commands on a clean checkout)
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features self-update -- -D warnings
cargo clippy --all-targets --features app-store -- -D warnings   # macOS only
cargo test --all-targets --features self-update
```

Rust tests live next to the code in `#[cfg(test)]` modules. Tests must never use the real OS keychain. Construct crypto/storage with an injected key and a temporary directory (`tempfile`).

## Guidelines

- Keep the backend⇄frontend contract consistent. Command args and payloads are snake_case, and commands reject with a plain string. If you change a command, update both `src-tauri/src/lib.rs` and its callers in `src/`.
- Never log clipboard contents, previews or key material.
- Anything stored in SQLite that contains user content must go through `crypto.rs`.
- Platform-specific code belongs in `platform.rs` behind `#[cfg(target_os = …)]`, with a defined fallback for the other platforms.
- Do not add network calls. The only network use is the opt-in update check.

## Commits and branches

Branch from `master`. Use conventional-style commit messages (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `ci:`).

## Releases

See [Building and releasing](./deployment.md).

## Security issues

Report vulnerabilities through [GitHub private security advisories](https://github.com/mordamax/clipsa/security/advisories/new), not public issues.
