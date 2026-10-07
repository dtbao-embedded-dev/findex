# CLAUDE.md — findex

Authoritative guidance for Claude Code in this repository. Read it before changing
any file. These rules are not suggestions; a change that breaks one is wrong even
if it works.

- **Project**: findex — A desktop launcher that finds files and folders by name, searches keywords inside file contents, tags folders, and opens results in a terminal or any app.
- **Type**: Tauri desktop app (`tauri`)
- **Version**: v0.1.0
- **License**: MIT © 2026 dtbao

---

## Branch rule

| Branch | Role |
|--------|------|
| `main` | Released code only. Protected — never commit or force-push here. |
| `developing` | Integration branch. Everything reaches `main` through it. |
| `feat/*`, `fix/*` | Short-lived work branches, cut from `developing`. |

A change lands on a work branch, merges into `developing`, and reaches `main` only
through a release.

---

## Code rule

Application code is split into `model/`, `view/` and `presenter/`. Each file is a module of functions — no god-classes, no logic in the view.

- One concern per module. A module exports functions; it does not export a class
  that exists only to hold them.
- Data flows one way: the view renders what the presenter hands it, the presenter
  calls the model, the model owns the rules. The view never reaches into the model.
- A function that needs more than its arguments to be understood is doing two jobs.
- Match the style already present in the file being edited.

### Tauri

- A `#[tauri::command]` in `src-tauri/src/commands.rs` takes its arguments and calls
  one function in `src-tauri/src/model/`. Business rules never live in `lib.rs` or in
  a command.
- Every command returns `AppResult<T>` from `src-tauri/src/error.rs`; the frontend
  receives the error as its message string. No `unwrap`/`expect` outside tests —
  `Cargo.toml` `[lints]` denies them.
- The frontend reaches Rust only through `src/model/`: a view or presenter never calls
  `invoke` itself.
- `app.security.csp` in `tauri.conf.json` stays a strict string. Never set it to
  `null`; widen the one directive a feature needs instead.
- A plugin's permissions go in `src-tauri/capabilities/default.json`, the narrowest
  that work.
- `rust-toolchain.toml` and `toolchain:` in both workflows move together.
- Commit `package-lock.json`/`pnpm-lock.yaml` and `src-tauri/Cargo.lock`:
  `tool-release.py` keeps the crate's version in `Cargo.lock` in step only when it is
  tracked.


---

## Version rule

[Semantic Versioning](https://semver.org/) with a `v` prefix: `vMAJOR.MINOR.PATCH`.

| Bump | When |
|------|------|
| MAJOR | A breaking change to a public interface, format or CLI |
| MINOR | New behaviour, backwards compatible |
| PATCH | Bug fix or internal change with no interface impact |

Below `v1.0.0` the interface is not yet stable: breaking changes bump MINOR.

The version lives in these files and nowhere else — `docs/scripts/tool-release.py`
rewrites all of them, so never edit one by hand:

- `package.json`
- `src-tauri/Cargo.toml`
- `README.md` (the version badge)

---

## Changelog rule

Write every change into `docs/CHANGELOG/Unreleased.md` **as it lands**, under
`### Added`, `### Changed`, `### Fixed` and `### Removed`. The version is decided
later; the file needs no version in its name to be written. Reconstructing a release
from git history afterwards always loses the *why*.

Write what changed for someone using this project, not a copy of the commit subjects,
and say *why* where it is not obvious. Before a release, check nothing was missed:

```bash
git log --no-merges --pretty=format:'%h %s' $(git describe --tags --abbrev=0)..HEAD
```

At release time `tool-release.py` renames `Unreleased.md` to `<tag>.md` (for example
`v0.2.0.md`) and leaves a fresh skeleton behind. It refuses a release whose notes hold
no `- ` entry.

---

## Release rule

Cutting `v0.2.0`, in this order:

1. **Check the notes.** `docs/CHANGELOG/Unreleased.md` covers every change since the
   last tag, per the changelog rule above.
2. **Dry run.** `python docs/scripts/tool-release.py v0.2.0 --dry-run` — read what
   it intends to touch.
3. **Release.** `python docs/scripts/tool-release.py v0.2.0`. The script then:
   - renames `Unreleased.md` to `v0.2.0.md`
   - rewrites the version badge in `README.md`
   - rewrites every version file listed above, and the crate's own entry in a
     tracked `Cargo.lock` beside a `Cargo.toml`
   - commits exactly those files as `chore(release): v0.2.0`
   - tags the release commit `v0.2.0`
   - merges the current branch into `developing`, then `developing` into `main`
4. **Push, once the user has authorized it.** The script prints the exact commands
   and pushes only with `--push`. Never push without being asked to in that turn.

The script refuses rather than guesses: a dirty tree, empty release notes, an
existing tag, a version that does not increase, or a release started from `main`
all stop it before it writes anything. The release workflow runs
`tool-release.py --check-tag <tag>` first, so a tag pushed by hand onto an
unreleased commit fails before anything is built.

---

## Commit rule

Use the `skill-support-commit` skill for every commit. It reads the diff, groups
the files into logical [Conventional Commits](https://www.conventionalcommits.org/)
and lands them.

```
<type>[(<scope>)]: <description>
```

Allowed types: `feat` `fix` `refactor` `style` `docs` `perf` `test` `build` `ci` `chore`.

The one exception is the release commit — `tool-release.py` writes
`chore(release): v0.2.0` itself, because that subject is parsed by tooling and must
never be paraphrased.

Never `git add .` or `git add -A`. Never `--no-verify`. Never rewrite pushed history.

---

## Memory rule

Project memory lives in `docs/memory-ai/`, maintained by the `skill-memory-ai`
skill. Read it at the start of a session to recover decisions, architecture notes
and past bug fixes; update it when a decision is made that the code alone does not
explain.

---

## Do not

- Do not commit directly to `main`
- Do not edit a version file or the README badge by hand — run `tool-release.py`
- Do not tag a release by hand — `tool-release.py` writes the notes, the versions and the tag together
- Do not push without being asked to in that turn
- Do not put business logic in the view layer
