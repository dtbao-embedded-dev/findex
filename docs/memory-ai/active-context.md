---
title: Active Context
updated: 2026-10-07
---

# Active Context

> Repository just scaffolded; the next focus is the first real feature.

## Current focus

Initial setup of findex; no feature work started.

## Recent changes

- 2026-10-07: scaffolded (tauri, react, pnpm, tray/updater/sqlite/titlebar); lockfiles committed.

## Next steps

1. Run `pnpm tauri dev` to confirm the window, tray and title bar.
2. Generate the updater signing key (deferred by the user on 2026-10-07).
3. Design name search, then the content index.

## Active decisions

- SQLite holds tags and the index; schema changes are append-only migrations. See
  [adr-0001](adr/adr-0001-scaffold-choices.md) and
  [database migrations](rule/database-migrations.md).

## Latest checkpoint

- None yet.
