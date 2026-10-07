---
title: Progress
updated: 2026-10-07
---

# Progress

> Scaffold only (v0.1.0, unreleased); no findex feature is built yet.

## What works

- Scaffold builds: pnpm typecheck, vitest (3 tests), vite build, cargo fmt, clippy
  `-D warnings`, cargo test (5 tests) all pass on 2026-10-07.
- Tray, single instance, custom title bar, SQLite with migrations, updater plugin wiring.

## What's left

1. Name search over files and folders
2. Content keyword search (decide FTS5 vs a dedicated engine first)
3. Folder tags (new migration)
4. Quick open in a terminal or any app (needs new capabilities)

See [product scope](architecture/product-scope.md).

## Known issues

- `main` branch protection must be enabled on GitHub by hand.

## Verification

- Build check above, plus `tool-release.py --selftest` and `--check-tag v0.1.0`: OK.
- Runtime (window, tray, title bar) not yet checked with `pnpm tauri dev`.
