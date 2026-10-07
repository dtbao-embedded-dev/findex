---
title: Search and Indexing Behavior
category: behavior
order: 1
purpose: What each unit of name search does — drive lifecycle, query ranking, stale-result handling, status wording, active drive and splash rules, reveal.
status: active
updated: 2026-10-07
source: src-tauri/src/model/index/service.rs, src-tauri/src/model/elevation.rs, src-tauri/src/model/search.rs, src-tauri/src/model/reveal.rs, src-tauri/src/model/drives.rs, src/presenter/searchPresenter.ts, src/presenter/statusPresenter.ts, src/presenter/drivePresenter.ts, src/presenter/splashPresenter.ts, src/view/SearchView.tsx
confidence: confirmed
verified_at: 2026-10-07
keywords: ranking, tier, latestOnly, keepSelection, highlight, summarize, watchStatus, ensure_active, default_letter, revealMain, lifecycle, gap, resume
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
- **Top-K:** a per-chunk max-heap of `limit` candidates is merged at the end. Paths are
  built only for the winners.
- **Cancellation:** checked at chunk start and every 1024 matches. A cancelled query
  returns `None`.

## Frontend

- **`latestOnly`:** each call takes a sequence number. An answer is used only if its
  number is still the latest and Rust did not return null. An empty query clears the
  results without IPC.
  - **Retry:** if the latest call gets null, it asks once more. Calls can reach Rust
    out of order, so an older search may have cancelled the newest one there.
- **Refresh on `total` change** (`SearchView`): re-runs the query on screen and keeps
  the selected path via `keepSelection`.
- **`highlight`:** marks the first occurrence of each non-filter term. It skips
  highlighting when lowercasing changes the name's length.
- **`describeResults`:** "N result(s) · X ms", or "Top 200 results" when the page is
  full.
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
