# Changelog

One file per release, named after its tag: `v0.1.0.md`, `v0.2.0.md`, ...

Changes are written into `Unreleased.md` as they land. `docs/scripts/tool-release.py`
renames it to `<tag>.md` at release time and leaves a fresh skeleton behind; it
refuses a release whose notes hold no `- ` entry.

Group the entries under `### Added`, `### Changed`, `### Fixed` and `### Removed`,
and write what changed for a reader of this project — not a copy of the subject lines.
Before releasing, check nothing was missed:

```bash
git log --no-merges --pretty=format:'%h %s' $(git describe --tags --abbrev=0)..HEAD
```
