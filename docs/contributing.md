# Contributing to Clipsa

Everyone taking part in Clipsa's issues, discussions and pull requests follows the [Code of Conduct](../CODE_OF_CONDUCT.md).

## Setup

Requirements: Rust stable (`rustup`), Node.js 22.17 or newer (SvelteKit 3 needs it), pnpm 9 and the [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/). On Debian/Ubuntu, CI installs:

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
pnpm site --check               # site pages generated from CHANGELOG.md are current (pnpm site to fix)
node scripts/check-changes.mjs  # release notes in .changes/ are valid
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features self-update -- -D warnings
cargo clippy --all-targets --features app-store -- -D warnings   # macOS only
cargo test --all-targets --features self-update
```

New functionality comes with tests that check its behaviour, and a bug fix comes with a test that fails without the fix. Rust tests live next to the code in `#[cfg(test)]` modules. Tests must never use the real OS keychain. Construct crypto/storage with an injected key and a temporary directory (`tempfile`).

## Fuzzing

`src-tauri/fuzz/` holds [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets for the code that reads clipboard values, stored items, settings changes and keys: `snapshot`, `decode`, `settings` and `crypto`. They call the functions in `src-tauri/src/fuzz.rs` (the `fuzz` feature). The **Fuzz** workflow runs each target for a minute on pull requests that change `src-tauri/`, and for ten minutes every week. A crash fails the run and attaches the input that caused it as the artifact `fuzz-crashes`. To run a target locally (needs nightly Rust and `cargo install cargo-fuzz`; on macOS add `-s none` after `run`, as AddressSanitizer builds don't link there):

```bash
cd src-tauri
cargo +nightly fuzz run settings
cargo +nightly fuzz run settings fuzz/artifacts/settings/<crash file>   # reproduce a crash
```

## Guidelines

- Keep the backend⇄frontend contract consistent. Command args and payloads are snake_case, and commands reject with a plain string. If you change a command, update both `src-tauri/src/lib.rs` and its callers in `src/`.
- Never log clipboard contents, previews or key material.
- Anything stored in SQLite that contains user content must go through `crypto.rs`.
- Platform-specific code belongs in `platform.rs` behind `#[cfg(target_os = …)]`, with a defined fallback for the other platforms.
- Do not add network calls. The only network use is the opt-in update check.

## Branches and pull requests

`master` only changes through pull requests, for maintainers too: direct pushes, force pushes and deletion are blocked. A PR can merge once CI passes on Ubuntu, macOS and Windows and the **Site** check passes; it is squash-merged, so `master` stays linear, and the branch is deleted. Branch from `master` and use conventional-style commit messages and PR titles (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `ci:`, `deps:`).

**Release notes:** if users will notice your change, add a note in `.changes/` describing what changes for them (see [`.changes/README.md`](../.changes/README.md)). Don't edit `CHANGELOG.md`; the release PR writes it from the notes.

Dependabot opens one grouped PR per ecosystem (GitHub Actions, Cargo, npm) each week, majors included; see `.github/dependabot.yml`.

## Releases

Releases are automated: notes in `.changes/` are collected into a "release: X.Y.Z" pull request, and merging it publishes the release. See [Releases](./deployment.md#releases).

## Security issues

Report vulnerabilities through [GitHub private security advisories](https://github.com/mordamax/clipsa/security/advisories/new), not public issues.
