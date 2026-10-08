---
title: Active Context
updated: 2026-10-07
---

# Active Context

> Second run done on `release/v0.1` (63a3dea..5782ca5): one active drive, right-click
> actions, file details, folder tags, auto update. Waiting on Administrator checks and
> a first published release.

## Current focus

Branch `feat/toolbar-themes-quick-tag` (from `developing` bc4864d, 8c10406..0197f51)
is complete and verified unelevated: search row (drive left, re-scan right, no
sidebar), Explorer "Tag with Findex" via the `findex-tag` helper and inbox, folder
sizes, Open with VS Code, recent searches, back-to-top, dark-only with six themes,
update button, footer copyright. Open: a real Explorer right-click check by the user;
merge into `developing` and any push wait for the user. (`feat/content-search-paging`
was merged into `developing` earlier, bc4864d.)

## Recent changes

- **2026-10-07, fifth run (`feat/toolbar-themes-quick-tag`):** 8c10406 search row;
  c2a9ef7 helper crate; ee16579 inbox; 6683abb drive-root fix; 83ce1da Explorer menu;
  76f3831 folder sizes; 443e803 VS Code; 4c4fa9e history; 0bcf7d2 back-to-top;
  0bba777 themes; 1672278 update button; 0197f51 copyright.

- **2026-10-07, fourth run (`feat/content-search-paging`):** 8992a8c Enter/dblclick;
  6de5172 named presets (migration 4); f1cf5bb Type filter; d30ce23 paging, no cap,
  limit setting removed; 8c870b6 content scanner; 3b9a520 dark segment fix; 9fec65f
  Contents mode; review fixes 488d425, bbf3c93.
- **2026-10-07, third run (`feat/ui-redesign-filters`):** 2a7e9c9 sidebar redesign;
  b69578b title bar version; dcf89ac filters; aca19fa results limit + virtual list;
  555cd11 tag catalog (migration 3); 6129b25 re-scan; d376f72 index RAM; review fixes
  3521159, d708739.
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

- **Layout:** no sidebar; drive picker left of the search field, re-scan right of it;
  "New tag" lives in Settings (user, 2026-10-07; replaced the Finder sidebar).
- **Appearance:** dark only; themes Graphite (default), Midnight, Aurora, Violet,
  Forest, Ember (user, 2026-10-07).
- **Quick tag outside the app:** Explorer right-click via a separate unelevated helper
  crate; the admin manifest stays. See [ADR-0003](adr/adr-0003-explorer-tag-helper.md).
- **Folder size:** measured automatically for visible rows (user, 2026-10-07).
- **Date filter:** dates are read from the disk for the best name matches; the index
  stays names-only (user, 2026-10-07).
- **Preset tags:** named, each with a colour: Working, Doc, Important, Todo, Review,
  Personal, Archive (user, 2026-10-07; replaced Finder's seven colours).
- **Enter / double-click:** do nothing (user, 2026-10-07).
- **Results:** no cap; names paged as the list scrolls, contents streamed (user,
  2026-10-07). The results-limit setting is gone.
- **Content search:** direct scan, streamed, no index (user, 2026-10-07).
- **RAM shown:** the index's own memory, not the process working set (user,
  2026-10-07).
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
