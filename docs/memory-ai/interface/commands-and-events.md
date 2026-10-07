---
title: Commands and Events
category: interface
order: 1
purpose: Every Tauri command, event and setting key the frontend and Rust share, with argument and result shapes.
status: active
updated: 2026-10-07
source: src-tauri/src/commands.rs, src-tauri/src/lib.rs, src-tauri/src/model/contents.rs, src-tauri/src/model/dated.rs, src-tauri/src/model/db.rs, src-tauri/src/tray.rs, src-tauri/src/model/drives.rs, src-tauri/src/model/find.rs, src-tauri/src/model/tags.rs, src-tauri/src/model/details.rs, src-tauri/src/model/launch.rs, src-tauri/src/model/search.rs, src/model/drives.ts, src/model/index.ts, src/model/search.ts, src/model/contents.ts, src/model/details.ts, src/model/tags.ts, src/model/launch.ts, src/model/updater.ts
confidence: confirmed
verified_at: 2026-10-07
keywords: invoke, command, event, index-status, check-updates, scan.drives, search_contents, Channel, Batch, Filters, exts, Answer, total, more, offset, indexBytes, rescan_drive, tag_catalog, create_tag, delete_tag, get_active_drive, set_active_drive, search_names, describe_paths, add_tag, open_terminal, open_claude, save_index, tag:
---

# Commands and Events

> The Rust ↔ frontend contract. Only `src/model/*.ts` names these; every command
> returns `AppResult<T>`, which the frontend receives as the error message string.

## Commands

| Command | Args | Returns | Model call |
|---------|------|---------|------------|
| `app_about` | — | `{name, version, copyright}` | `about::about` (copyright from build env `FINDEX_COPYRIGHT`) |
| `splash_ready` | — | `()` | `splash::finish` (once; shows main) |
| `list_drives` | — | `Drive[]` `{letter, label, filesystem, removable, total, free, method: "mft"\|"walk"}` | `drives::list` |
| `get_active_drive` | — | `string \| null` (null only when no drive exists) | `drives::load_active` |
| `set_active_drive` | `letter: string` | stored letter (upper case) | `IndexService::switch_to` (saves, then indexes that drive alone) |
| `index_status` | — | `Status` | `IndexService::status` |
| `rescan_drive` | — | the active letter | `IndexService::rescan` (a run with `rebuild`: never resumes from the journal; the old index stays searchable) |
| `save_index` | — | `()` | `IndexService::save_changed` (called before the updater installs) |
| `search_names` | `query: string, filters?: Filters, offset: number, count: number` | `Answer \| null` `{hits: Hit[], total: number \| null, more, partial, indexBytes}`; `Hit` = `{name, path, isDir}`; null = superseded by a newer search | `find::search` on `spawn_blocking`: one page (`count` rows from rank `offset`) from the index (or the tags table when tags are asked), then `dated::keep` for a date range. No cap (2026-10-07, d30ce23). |
| `cancel_search` | — | `()` | `IndexService::cancel_searches` (bumps the query counter): stops a running scan when the user clears the field or leaves Contents (488d425) |
| `search_contents` | `query: string, filters?: Filters, onBatch: Channel<Batch>` | `()` once the scan ended or was stopped; results arrive on the channel as `Batch` `{hits: Found[], filesRead, filesTotal, done}`; `Found` = `Hit` plus `line?`, `snippet?` (absent for a name match) | `contents::scan` on `spawn_blocking` (8c870b6) |
| `describe_paths` | `paths: string[]` | `Details[]` `{path, kind, size \| null, modified \| null (ms), tags}`; vanished paths omitted | `details::describe_tagged` |
| `reveal_path` | `path: string` | `()`; error if the path no longer exists | `reveal::reveal` (`explorer.exe /select,`) |
| `open_folder` | `path: string` | `()` | `launch::open_folder` (`explorer.exe "<dir>"`) |
| `open_terminal` | `path: string` | `()` | `launch::open_terminal` (wt.exe `-d .`, else `powershell.exe -NoExit`) |
| `open_claude` | `path: string` | `()`; error if `claude` is not on PATH | `launch::open_claude` (a terminal running the `claude` found on PATH, by full path) |
| `add_tag`, `remove_tag` | `path, tag` | the folder's tags, sorted | `tags::add`, `tags::remove` |
| `tag_catalog` | — | `TagInfo[]` `{tag, color \| null, preset, folders}`: the 7 named presets in fixed order, then custom tags by name | `tags::catalog` |
| `create_tag` | `tag` | the catalog; refuses an invalid or existing tag | `tags::create` |
| `delete_tag` | `tag` | the catalog; refuses a preset or unknown tag; takes the tag off every folder | `tags::delete` |
| `get_setting`, `set_setting` | key / key, value | scaffold; not called by the frontend | `Db` |

Removed on 2026-10-07: `get_scan_drives`, `set_scan_drives` and `start_index`, with
the setup screen; `list_tags` (replaced by `tag_catalog`, branch
`feat/ui-redesign-filters`); `get_results_limit` and `set_results_limit` with
`model/settings.rs` (branch `feat/content-search-paging`, d30ce23): there is no cap.

## `Filters` and `Answer` (`search_names`)

- **`Filters`** (camelCase, every field optional): `kind: "any"|"file"|"folder"`
  (replaces a typed `type:`), `tags: string[]` (added to typed `tag:`),
  `exts: string[]` (added to typed `ext:`, normalised the same way; f1cf5bb),
  `modifiedFrom`, `modifiedTo` (ms since the epoch, `to` excluded). The frontend
  measures preset dates such as "Today" in local time when each search is asked
  (`filterPresenter.dateRange`), so Rust needs no time zone.
- **Paging:** `search::run` counts every match and keeps the top `offset+count` per
  chunk; paths are built only for the page. `total` is exact, `more` =
  `offset + hits.len() < total`. Each page re-ranks the drive, so pages follow one
  order; the frontend (`searchPresenter.appendPage`) skips a path already shown.
- **Date range:** the index holds no dates. The name search asks for a pool of
  `max(clamp(wanted*10, 20 000, 100 000), wanted)` matches (`wanted = offset+count`),
  then `dated::keep` reads mtimes in rank order, 1 024 at a time in parallel, until
  `wanted + 1` are inside (the extra one sets `more`), checking the search's cancel
  token between batches. `total` is null unless every match was checked; `partial`
  is true when the pool was full and no further page was found.
- **`indexBytes`:** `IndexService::memory_bytes`, the sum of `VolumeIndex::heap_bytes`
  (container capacities); about 133 MB for 1.2 M names on E: (debug, 2026-10-07).

## Event `index-status`

The payload is a `Status` `{drives: DriveStatus[]}`, and each `DriveStatus` has:

| Field | Meaning |
|-------|---------|
| `letter` | The drive letter. |
| `state` | `"indexing" \| "ready" \| "failed"` |
| `entries` | The running build's progress, or the full count once the drive is ready. |
| `searchable` | What search sees now: a loaded snapshot, or the last build. |
| `method` | How the drive was actually read. |
| `note` | The reason for a fallback, a failure, or "Live updates stopped". |

It is sent on every start, at every 65,536 entries of progress, on install, and at most
once a second for live updates.

## Event `check-updates`

No payload. Emitted by the tray's "Check for updates" (`tray.rs`); the frontend runs a
manual update check (`createUpdater().check(true)`).

## Settings keys

| Key | Value |
|-----|-------|
| `scan.drives` | JSON array holding the one active drive, e.g. `["C"]`. Absent until the first launch, when Rust saves the system drive. An older multi-drive list keeps only its first letter. |
| `search.results_limit` | No longer read (removed in d30ce23); a value saved by an earlier build stays in the table, unused. |

## Tables

| Table | Columns |
|-------|---------|
| `setting` | `key TEXT PRIMARY KEY, value TEXT` (migration 1) |
| `folder_tag` | `path TEXT COLLATE NOCASE, tag TEXT, PRIMARY KEY (path, tag)`, index on `tag` (migration 2) |
| `tag` | `name TEXT PRIMARY KEY, color TEXT, preset INTEGER, ord INTEGER` (migration 3; existing `folder_tag` tags backfilled as custom; `tags::add` registers a new name as custom). Migration 4 (6de5172) replaces the colour presets with named ones: working (blue, ord 1), doc (green), important (red), todo (orange), review (yellow), personal (purple), archive (gray, ord 7); folder rows move to the same-colour preset, and a custom tag of the same name becomes the preset. |

## Query syntax (`search_names`)

- **Terms:** whitespace-separated, lowercased. All of them must appear in the name.
- **Extension filter:** `ext:pdf;docx` (`,` also works; a leading `.` is stripped).
- **Type filter:** `type:file|files|folder|folders|dir|dirs`. An unknown `type:x` stays
  a term.
- **Tag filter:** `tag:work;fw` (`,` also works; a leading `#` is stripped). Lists the
  folders carrying every listed tag, on any drive, from `folder_tag`, narrowed and
  ranked by the name terms (`search::rank_name`). Deleted folders are skipped.
- **Empty query:** a query with no term and no `ext:` lists nothing.
