---
title: Active Context
updated: 2026-10-07
---

# Active Context

> Second run done on `release/v0.1` (63a3dea..5782ca5): one active drive, right-click
> actions, file details, folder tags, auto update. Waiting on Administrator checks and
> a first published release.

## Current focus

The run "auto update, single-drive main screen, context menu, file details, folder
tags" is complete and verified unelevated. Human checks pending: see `progress.md`.

## Recent changes

- **2026-10-07, second run:**
  - 63a3dea block the WebView right-click menu.
  - b5e17c3 one active drive, toolbar picker (setup screen removed).
  - 01c024b right-click actions, unelevated launches.
  - af7290b type, size and date on rows.
  - 3e4de77, 735321a folder tags (migration 2).
  - 5782ca5 auto update.
  - Review fixes: b036323 (launch), 9ee21a0 (tags), b3e5637 (updater), e2bba88 (menu).
- **2026-10-07, first run:** splash, name index, snapshots, USN watcher (8299648..b80de4f).

## Next steps

1. **Administrator checks** in `progress.md` (MFT, USN, release UAC, unelevated
   terminal).
2. **First release** to exercise the updater end to end.
3. **Merge / push** of `release/v0.1`: not authorized yet.

## Active decisions

- **One drive at a time** (user, 2026-10-07): the old drive leaves memory; its `.fxi`
  stays for a quick return.
- **Launched tools run unelevated** (user): Explorer as parent; refuse rather than
  start elevated. See [actions, tags and updates](behavior/actions-tags-updates.md).
- **Open with Claude = Claude Code CLI** in a terminal at the folder (user).
- **Updates:** download in the background, ask to restart (user).
- **Privileges:** always `requireAdministrator` for release builds. See
  [ADR-0002](adr/adr-0002-name-index.md).
- **UI:** follows the `apple-design` skill.
- **Storage:** SQLite, append-only migrations. See
  [database migrations](rule/database-migrations.md).
