---
title: Commands and Events
category: interface
order: 1
purpose: Every Tauri command, event and setting key the frontend and Rust share, with argument and result shapes.
status: active
updated: 2026-10-07
source: src-tauri/src/commands.rs, src-tauri/src/lib.rs, src-tauri/src/model/about.rs, src-tauri/src/model/drives.rs, src-tauri/src/model/index/service.rs, src-tauri/src/model/search.rs, src/model/about.ts, src/model/drives.ts, src/model/index.ts, src/model/search.ts, src/model/splash.ts
confidence: confirmed
verified_at: 2026-10-07
keywords: invoke, command, event, index-status, scan.drives, search_names, start_index, reveal_path, splash_ready, app_about
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
| `get_scan_drives` | — | `string[] \| null` (null before setup) | `drives::load_selection` |
| `set_scan_drives` | `letters: string[]` | stored letters (upper case, unique, sorted; ≥1 or error) | `drives::save_selection` |
| `start_index` | — | `()` | `IndexService::start_saved` |
| `index_status` | — | `Status` | `IndexService::status` |
| `search_names` | `query: string, limit?: number` (default 200) | `Hit[] \| null` `{name, path, isDir}`; null = superseded by a newer search | `IndexService::search` on `spawn_blocking` |
| `reveal_path` | `path: string` | `()`; error if the path no longer exists | `reveal::reveal` (`explorer.exe /select,`) |
| `get_setting`, `set_setting` | key / key, value | scaffold; not called by the frontend | `Db` |

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

## Settings keys

| Key | Value |
|-----|-------|
| `scan.drives` | JSON array of drive letters, e.g. `["C","E"]`; absent means setup is needed |

## Query syntax (`search_names`)

- **Terms:** whitespace-separated, lowercased. All of them must appear in the name.
- **Extension filter:** `ext:pdf;docx` (`,` also works; a leading `.` is stripped).
- **Type filter:** `type:file|files|folder|folders|dir|dirs`. An unknown `type:x` stays
  a term.
- **Empty query:** a query with no term and no `ext:` lists nothing.
