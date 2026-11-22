# Release notes

Every pull request that changes something users notice adds one note here: a
Markdown file with any name, for example `picker-empty-search.md`:

```md
---
"clipsa": patch:fixed
---

The quick picker opens with an empty search box every time, instead of keeping your previous search.
```

- **First line:** how big the change is: `patch` (fixes and small changes),
  `minor` (new features) or `major`. Clipsa stays below 1.0, so use `patch`
  or `minor`.
- **After the colon:** the changelog section: `added`, `changed`, `fixed`,
  `removed` or `security`.
- **The note** is published as is in the changelog on
  [clipsa.dev/changelog](https://clipsa.dev/changelog), the GitHub release and
  Clipsa's update window. Write it for people who use Clipsa: what they will
  notice, and anything they need to do. Leave implementation details for the
  pull request description. Keep it to one paragraph; `**bold**`, `` `code` ``
  and links work.

Pull requests that users don't notice (documentation, CI, tests, refactoring)
add no note.

CI checks the notes (`node scripts/check-changes.mjs`) and lists them in the
**Site** check's summary. When notes reach `master`, the **Release PR**
workflow opens (or updates) a pull request "Release X.Y.Z" that bumps the
version, moves the notes into `CHANGELOG.md` and regenerates the website.
Merging it tags and publishes the release. See
[docs/deployment.md](../docs/deployment.md#releases).
