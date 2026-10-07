---
title: Active Context
updated: 2026-10-07
---

# Active Context

> Name search is built end to end; the next step is the admin-only verification, then
> the next capability (content search).

## Current focus

The run "splash + drive setup + fast name search (omni-search concept)" is done on
`release/v0.1`. It is waiting on the human-executed elevated checks listed in
`progress.md`.

## Recent changes

- **2026-10-07:**
  - T1 splash (8299648).
  - T2 index and query engine (f18fbc1).
  - T3 drive setup (f1fe9c3).
  - T4 MFT / walk / service / admin manifest (e411593).
  - T5 search screen (15ef234).
  - T6 snapshots (8c782bf, ac70f51, 453a001).
  - T7 USN watcher (aff652c).
  - Review fixes (2a78413, 81038e1, 3d34094, b80de4f).

## Next steps

1. **Elevated checks:** run the three checks in an Administrator terminal.
2. **Merge:** decide merge or push of `release/v0.1`. Release branches normally take
   fixes only.
3. **Optional:** a compact snapshot encoding, and a streaming walk to cut its memory
   peak.

## Active decisions

- **Privileges:** always `requireAdministrator` for release builds. The user chose
  this over a hybrid. See [ADR-0002](adr/adr-0002-name-index.md).
- **Live updates:** included in this run (user choice).
- **UI:** follows the `apple-design` skill (user request).
- **Storage:** SQLite holds settings, and the schema changes only by append-only
  migrations. See [adr-0001](adr/adr-0001-scaffold-choices.md) and
  [database migrations](rule/database-migrations.md).

## Latest checkpoint

- 2026-10-07 `name-search-splash-setup-2026-10-07`: T1–T7 plus review fixes; admin checks pending.
