<div align="center">

<img src="src-tauri/icons/128x128@2x.png" alt="Findex icon" width="128" height="128">

# Findex

![Version](https://img.shields.io/badge/version-v0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB)
![Rust](https://img.shields.io/badge/Rust-stable-DEA584)

</div>

A desktop launcher that finds files and folders by name, searches keywords inside file contents, tags folders, and opens results in a terminal or any app.

## Background

<!-- Why this exists: the problem, who it is for, and what makes it different from
the obvious alternatives. Two or three short paragraphs. -->

Findex is a Tauri desktop app by dtbao.

## Install

### Requirements

| Tool | Version | Notes |
|------|---------|-------|
| Node.js | 22+ |  |
| pnpm | current | the package manager this repository was set up with |
| Rust | stable | pinned by `rust-toolchain.toml` |
| WebView2 | any | preinstalled on Windows 10 and 11 |

### From source

```bash
git clone https://github.com/dtbao-embedded-dev/findex
cd findex
pnpm install
```

## Usage

```bash
pnpm run tauri dev
```

```text
The app window opens; edits under src/ reload it.
```

## Build from source

```bash
pnpm run tauri build
```

Run the same checks CI runs before opening a pull request:

```bash
pnpm run typecheck
pnpm test
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

The CI workflow is the authoritative build recipe.

## Project structure

```text
src/
  model/       rules and data, invoke wrappers
  presenter/   calls the model, hands state to the view
  view/        renders only
src-tauri/src/
  commands.rs  thin #[tauri::command] wrappers
  error.rs     the one error type every command returns
  model/       business rules, unit-tested
docs/          changelog, release script, memory bank
```

Application code is split into `model/`, `view/` and `presenter/`. Each file is a module of functions — no god-classes, no logic in the view.

## Contributing

1. Branch from `developing` as `feat/<name>` or `fix/<name>`; `main` holds released
   code only.
2. Commit with [Conventional Commits](https://www.conventionalcommits.org/) and note
   the change in [`docs/CHANGELOG/Unreleased.md`](docs/CHANGELOG/Unreleased.md).
3. Make sure the checks above pass, then open a pull request against `developing`.

Releases follow [Semantic Versioning](https://semver.org/) and are cut with
`python docs/scripts/tool-release.py vX.Y.Z`; each one has its notes in
[`docs/CHANGELOG/`](docs/CHANGELOG/). [CLAUDE.md](CLAUDE.md) holds the full rules.

## License

[MIT](LICENSE) © 2026 dtbao
