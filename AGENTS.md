# Working on Clipsa

Read docs/contributing.md first: setup, the checks CI runs, and code rules.

- Branch from master. PR titles use conventional prefixes (fix:, feat:, docs:, ci:, refactor:, test:, deps:).
- If users will notice the change, add a release note in .changes/ (format and writing rules: .changes/README.md).
  Write it for people who use Clipsa: what they notice and anything they must do. Docs, CI, test and refactoring
  changes get no note. Never edit CHANGELOG.md or the version in src-tauri/Cargo.toml: the release PR does that.
- Before opening a PR, run the checks in docs/contributing.md, including node scripts/check-changes.mjs.
- Never log clipboard contents, previews or key material. User content stored in SQLite goes through crypto.rs.
- Don't add network calls. Tests never use the real OS keychain.
- Security model and release pipeline: docs/security.md, docs/deployment.md.
