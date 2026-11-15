# Contributing to Clipsa

## Setup

```bash
git clone https://github.com/mordamax/clipsa.git
cd clipsa
pnpm install
pnpm tauri dev
```

Requirements: Rust stable, Node.js 22, pnpm 9 and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/). On Linux, also install `libssl-dev libdbus-1-dev libxdo-dev`.

Debug builds store data in `<data dir>/clipsa-dev/` with a file-based key. Development never touches your real history or the keychain.

## Before opening a PR

Run the same checks as CI:

```bash
pnpm check
pnpm build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
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

See [deployment.md](./deployment.md).

## Security issues

Report vulnerabilities through [GitHub private security advisories](https://github.com/mordamax/clipsa/security/advisories/new), not public issues.
