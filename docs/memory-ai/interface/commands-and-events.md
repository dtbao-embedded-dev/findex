---
title: Commands and Events
category: interface
order: 1
purpose: Every Tauri command, event and setting key the frontend and Rust share, with argument and result shapes.
status: active
updated: 2026-10-07
source: src-tauri/src/commands.rs, src-tauri/src/lib.rs, src-tauri/src/tray.rs, src-tauri/src/model/drives.rs, src-tauri/src/model/find.rs, src-tauri/src/model/tags.rs, src-tauri/src/model/details.rs, src-tauri/src/model/launch.rs, src-tauri/src/model/search.rs, src/model/drives.ts, src/model/index.ts, src/model/search.ts, src/model/details.ts, src/model/tags.ts, src/model/launch.ts, src/model/updater.ts
confidence: confirmed
verified_at: 2026-10-07
keywords: invoke, command, event, index-status, check-updates, scan.drives, get_active_drive, set_active_drive, search_names, describe_paths, add_tag, open_terminal, open_claude, save_index, tag:
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
| `save_index` | — | `()` | `IndexService::save_changed` (called before the updater installs) |
| `search_names` | `query: string, limit?: number` (default 200) | `Hit[] \| null` `{name, path, isDir}`; null = superseded by a newer search | `find::search` on `spawn_blocking` (the index, or the tags table for `tag:`) |
| `describe_paths` | `paths: string[]` | `Details[]` `{path, kind, size \| null, modified \| null (ms), tags}`; vanished paths omitted | `details::describe_tagged` |
| `reveal_path` | `path: string` | `()`; error if the path no longer exists | `reveal::reveal` (`explorer.exe /select,`) |
| `open_folder` | `path: string` | `()` | `launch::open_folder` (`explorer.exe "<dir>"`) |
| `open_terminal` | `path: string` | `()` | `launch::open_terminal` (wt.exe `-d .`, else `powershell.exe -NoExit`) |
| `open_claude` | `path: string` | `()`; error if `claude` is not on PATH | `launch::open_claude` (a terminal running the `claude` found on PATH, by full path) |
| `add_tag`, `remove_tag` | `path, tag` | the folder's tags, sorted | `tags::add`, `tags::remove` |
| `list_tags` | — | `TagCount[]` `{tag, folders}`, most used first | `tags::all` |
| `get_setting`, `set_setting` | key / key, value | scaffold; not called by the frontend | `Db` |

Removed on 2026-10-07: `get_scan_drives`, `set_scan_drives` and `start_index`, with
the setup screen.

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

## Tables

| Table | Columns |
|-------|---------|
| `setting` | `key TEXT PRIMARY KEY, value TEXT` (migration 1) |
| `folder_tag` | `path TEXT COLLATE NOCASE, tag TEXT, PRIMARY KEY (path, tag)`, index on `tag` (migration 2) |

## Query syntax (`search_names`)

- **Terms:** whitespace-separated, lowercased. All of them must appear in the name.
- **Extension filter:** `ext:pdf;docx` (`,` also works; a leading `.` is stripped).
- **Type filter:** `type:file|files|folder|folders|dir|dirs`. An unknown `type:x` stays
  a term.
- **Tag filter:** `tag:work;fw` (`,` also works; a leading `#` is stripped). Lists the
  folders carrying every listed tag, on any drive, from `folder_tag`, narrowed and
  ranked by the name terms (`search::rank_name`). Deleted folders are skipped.
- **Empty query:** a query with no term and no `ext:` lists nothing.
