---
title: Search and Indexing Behavior
category: behavior
order: 1
purpose: What each unit of name search does — drive lifecycle, query ranking, stale-result handling, status wording, active drive and splash rules, reveal.
status: active
updated: 2026-10-07
source: src-tauri/src/model/index/service.rs, src-tauri/src/model/dated.rs, src-tauri/src/model/find.rs, src-tauri/src/model/contents.rs, src/presenter/contentPresenter.ts, src-tauri/src/model/index/store.rs, src/presenter/filterPresenter.ts, src/presenter/listPresenter.ts, src-tauri/src/model/elevation.rs, src-tauri/src/model/search.rs, src-tauri/src/model/reveal.rs, src-tauri/src/model/drives.rs, src/presenter/searchPresenter.ts, src/presenter/statusPresenter.ts, src/presenter/drivePresenter.ts, src/presenter/splashPresenter.ts, src/view/SearchView.tsx
confidence: confirmed
verified_at: 2026-10-07
keywords: ranking, tier, filters, date filter, partial, rescan, resume_point, paging, total, more, content search, scan, Contents mode, heap_bytes, latestOnly, keepSelection, highlight, summarize, watchStatus, ensure_active, default_letter, revealMain, lifecycle, gap, resume
---

# Search and Indexing Behavior

> How a drive goes from chosen to searchable to live, and how a typed query becomes
> the rows on screen.

## Drive lifecycle (`IndexService::index_drive`, one thread per drive)

1. **Load the snapshot.** `<L>.fxi` is installed and `searchable` is set. Any error
   other than "not found" is logged and ignored.
   - **Restart with the drive still chosen:** the in-memory index and its journal
     position from `positions` are kept instead, because they are newer.
2. **Resume.** This needs NTFS and `watch::continues`: the same journal id, and a
   `first_usn ≤ saved ≤ next_usn`. If both hold, the drive is marked Ready from the
   snapshot.
3. **Otherwise build.**
   - **NTFS:** read the MFT. If that fails (no admin), walk the folders instead, with a
     "Folder scan · …" note (`fallback_note`): missing rights say "open Findex as
     administrator…", other errors carry their reason.
   - **Other filesystems:** walk.
   - **Then:** save the snapshot (no lock held), install it, and mark the drive Ready.
4. **Follow (NTFS with a journal).** Batches are applied under the write lock.
   - **Status:** sent at most once a second.
   - **Saving:** the snapshot is saved every 10 minutes while dirty.
   - **Ending:** a journal gap goes back to step 3. A newer `start` stops the loop
     (`run` token). A failure sets the note "Live updates stopped".

- **`start`** drops drives no longer chosen. Every install, status update or save
  from an older `run` is discarded.
- **Cancelling an older run:** the MFT read checks `keep_going` every 65,536 entries,
  and the walk checks it before each folder. A stale watcher's batch is rejected
  under the volumes write lock.
- **Saving:** snapshot saves take one `saving` mutex.
- **Off the main thread:** `start_index` and `list_drives` run on `spawn_blocking`.

## Query (`search::run`)

- **Finding candidates:** the longest term is searched with `memchr::memmem` across
  1 MiB arena chunks in parallel (rayon). Each hit maps to its node by `owner_of`.
  Dead strings have no owner. Scanning resumes after the name's `\0`.
- **Checks:** the other terms must each be found in the name. `ext:` compares the text
  after the last `.`. `type:` filters folders or files.
- **Tier per term:**

  | Tier | Match |
  |------|-------|
  | 3 | The whole name, or the name without its extension |
  | 2 | A prefix |
  | 1 | Start of a word (the previous char is not alphanumeric) |
  | 0 | Anywhere else |

  Terms are summed.
- **Order:** tier descending, then shorter name (in chars), then folders first, then
  alphabetical, then volume and id.
- **Top-K and count:** a per-chunk max-heap of `offset+count` candidates is merged at
  the end; every chunk also counts its matches, so the total is exact. Paths are built
  only for the requested page (d30ce23). A deep page holds `offset+count` candidates
  per chunk, so its memory grows with how far the list was scrolled.
- **Cancellation:** checked at chunk start and every 1024 matches. A cancelled query
  returns `None`.

## Filters, paging, re-scan and memory (2026-10-07)

- **Filters:** `search::Query::narrow` merges the filter bar (kind replaces `type:`,
  tags join `tag:`, file types join `ext:`). A date range goes through `dated::keep`
  on a larger name pool; see [commands and events](../interface/commands-and-events.md).
  A date or kind with no name lists nothing (`canSearch`); a tag alone lists tagged
  folders, and a type alone every file of that type (f1cf5bb).
- **Cancellation:** `IndexService::begin_search` bumps the query counter and returns
  the "replaced?" check, passed to `search::run` and to `dated::keep`. A tag query
  also begins a search, so it cancels an older index search.
- **Measured (debug build, E: walk, 1.18 M names):** `report` + Last 30 days 36 ms;
  `e` + last day (pool of 20 000 checked) about 0.8 s with `partial`.
- **No cap (branch `feat/content-search-paging`, d30ce23):** the results limit
  setting is gone. The frontend asks `PAGE` = 500 rows, then the next page when
  `needsMore(shown.last, loaded, more)` (within 100 rows of the end). Checked over CDP
  on E: `e` reported 849,123 results and loaded 25 contiguous pages (12,500 unique
  paths); `readme` loaded to its total, 11,555 rows. A background refresh asks
  `max(PAGE, rows loaded)` so it never shortens the list.
- **Enter and double-click** select only; they open or reveal nothing (8992a8c).
  Every action goes through the right-click menu.
- **Re-scan:** `IndexService::rescan` starts a run with `rebuild`; `resume_point`
  returns `None` for it, so an NTFS drive is read from its MFT again instead of
  resuming from the journal. `load_snapshot` keeps the installed index searchable
  meanwhile (checked: 200 hits for `findex` during a 4 s re-read of E:).
- **Memory:** `VolumeIndex::heap_bytes` sums container capacities; 133 MB for E:
  against a 191 MB working set (156 MB private) for the debug app.

## Frontend

- **`latestOnly`:** returns `{search, more, stop}` sharing one sequence number.
  `search` takes a new number; an answer is used only if it is still the latest and
  Rust did not return null. `more(query, filters, loaded)` asks the page after
  `loaded` under the current number and is dropped once a newer search started;
  `stop` bumps the number when a content search takes over. `more` asks nothing while
  a newer search has not answered (`answered !== latest`, bbf3c93), so two queries'
  rows never mix. An empty query clears the results without IPC.
  - **Retry:** if the latest call gets null, it asks once more. Calls can reach Rust
    out of order, so an older search may have cancelled the newest one there.
- **Refresh on `total` change** (`SearchView`): re-runs the query on screen and keeps
  the selected path via `keepSelection`.
- **`highlight`:** marks the first occurrence of each non-filter term. It skips
  highlighting when lowercasing changes the name's length.
- **`describeResults`:** "N result(s) · X ms" with N the exact total; "N+ results"
  when a date filter left the total unknown and more pages follow.
- **`summarize`:**
  - **No drives:** "No drives chosen".
  - **While indexing:** "Indexing C: n · E: n".
  - **Otherwise:** "N items on C: E:", plus "· F: failed".
  - **Count:** the total sums `searchable`.
- **`watchStatus`:** subscribes first, then reads. The read is dropped if an event
  already arrived.
- **Active drive** (since 2026-10-07; replaced the setup screen):
  - **One at a time:** `IndexService::start_saved` indexes only `drives::ensure_active`.
    On the first launch that is `%SystemDrive%`, else the first fixed NTFS drive, else
    the first drive (`default_letter`), saved at once.
  - **Legacy list:** a saved `["C","E"]` loads as C:; E:'s `.fxi` stays on disk.
  - **Switching:** `switch_to` saves the letter, then `start([letter])`, which drops
    the other drive from memory. The picker lists the drives again on every open.
- **Splash:** `revealMain` waits for the routing and for 900 ms, whichever is later,
  then calls `splash_ready`. A failure is ignored, because the Rust fallback shows
  main.
- **Reveal:** refused when the path no longer exists; otherwise
  `explorer.exe /select,"<path>"`, passed with `raw_arg`.

## Content search (branch `feat/content-search-paging`, 8c870b6, 9fec65f)

- **Engine (`contents::scan`):** no index. It begins its own search token, sends the
  name matches first (`find::answer` with `count = usize::MAX`, 500 per batch), then
  collects candidate paths with `IndexService::files_where` under the volumes read
  lock and releases it before any file is opened. Files are read on a rayon pool of
  `min(cores, 8)` threads; every term must match (`regex::bytes`, case-insensitive,
  Unicode); the first term's first match gives the line and a snippet (40 chars
  before, 160 in all). Skipped: over 8 MiB, a NUL in the first 8 KiB, types outside
  the text allowlist (unless the Type filter names them), folders, and name matches
  already sent. Tags keep files under the tagged folders; a date is checked on the
  file's metadata.
- **Batches:** flushed at 64 hits or 150 ms after each file looked at (read or
  skipped, 488d425); the last one has `done: true`. A stopped scan sends nothing more
  (tested by mutation: removing the guard fails `a_stopped_scan_sends_nothing_more`).
  A `begin_search` landing between the check and the send can let one batch through;
  the frontend gate (`mine !== latest`) is what drops it.
- **Stopping:** `latestScan` calls `cancel_search` when a running scan is stopped or
  the field is cleared (only then, so a name search is never cancelled by it).
  Checked: 42 MB read in 2 s while scanning E:, 0 MB in the 2 s after clearing.
- **Frontend (`contentPresenter.latestScan`):** waits 250 ms after typing, drops
  batches of a superseded scan, grows one array in place (a `Set` of paths skips
  repeats) and copies it only when it redraws, at most every 100 ms plus once at the
  end. `SearchView` never auto-refreshes a content
  search on index changes.
- **Measured (debug, unelevated, E:):** 49,585 `.c` files read in 36.7 s cold and
  3.3 s warm; with `esp_event_base_t`, 204 rows were on screen after 1 s of reading.
  The whole of E: holds 472,992 text-type files, so an unfiltered scan takes minutes
  cold.
- **Known costs:** a one-letter content query lists every name match first (e.g.
  ~680 k for `a` on E:) before reading any file; a dated content query reads the
  mtime of every name match.
