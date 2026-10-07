---
title: "ADR-0002: Name Index"
category: architecture
purpose: How findex indexes file and folder names — MFT read as administrator, folder walk fallback, in-memory arena search, snapshots and USN live updates.
status: active
updated: 2026-10-07
source: src-tauri/src/model/index/mft.rs, src-tauri/src/model/index/snapshot.rs, src-tauri/src/model/index/watch.rs, src-tauri/src/model/index/walk.rs, src-tauri/src/model/index/service.rs, src-tauri/src/model/index/store.rs, src-tauri/src/model/search.rs, src-tauri/src/model/win32.rs, src-tauri/build.rs, src-tauri/findex.manifest
confidence: confirmed
verified_at: 2026-10-07
keywords: mft, usn, FSCTL_ENUM_USN_DATA, requireAdministrator, walker, rayon, memmem, omni-search, name search
---

# ADR-0002: Name Index

> Names are read from the NTFS file table (admin), or by walking folders, into one
> in-memory arena per drive that is searched with SIMD substring passes in parallel.

## Context

The user asked for fast name search modelled on github.com/Eul45/omni-search. Read on
2026-10-07, its fast path is C++:
- **Reading:** `FSCTL_ENUM_USN_DATA` over the MFT, which needs `requireAdministrator`.
- **Fallback:** a `FindFirstFileExW` walk.
- **Live updates:** a USN journal poller.
- **Query side:** a single-threaded substring scan over full-path strings, with no
  ranking and a stat call per result.

## Decision

| Choice | Reason |
|--------|--------|
| Release exe embeds `requireAdministrator` (`build.rs`, release profile only) | User chose it over a hybrid; MFT read needs it. Debug keeps `asInvoker` so `tauri dev`/`cargo test` run unelevated. |
| NTFS → MFT; failure or non-NTFS → parallel `read_dir` walk | Walk never enters junctions/symlinks, so link loops cannot hang it. |
| One lowercase arena per volume, `\0`-separated, nodes hold parent FRN | One memmem pass per 1 MiB chunk (rayon); paths built only for the top-K. Moves/renames touch one node. |
| Ranking tiers: exact/stem 3, prefix 2, word start 1, substring 0; then shorter, folders, alphabetical | Deterministic, tested ordering. |
| Latest-query-wins: generation counter checked per chunk | A typed character cancels the previous search. |
| All `unsafe` in `model/win32.rs` only (`#![allow(unsafe_code)]` there) | Crate-wide `unsafe_code = "deny"` stays. |

## Measured

- **1M synthetic names** (`perf_one_million`, release): p95 4 ms for "report", 15 ms
  for "e" (matches almost every name).
- **Running the tests and benchmark:** `cargo test --release --lib`. Without `--lib`,
  the release app binary's test harness may require elevation.

## Persistence and live updates

| Choice | Reason |
|--------|--------|
| Snapshot per drive, `<app data>/index/<L>.fxi`: magic `FXI1`, version, journal position, live entries, FNV-1a trailer; temp file then rename | Search works at launch before any scan; a damaged or other-version file is ignored and rebuilt |
| Snapshot written before install, never under the volumes lock | Saving under the read lock queued the next install and every search behind it |
| NTFS: resume from the snapshot's USN when the journal id matches and `first_usn <= saved <= next_usn` | Skips a full rebuild at launch |
| Watcher polls `FSCTL_READ_USN_JOURNAL` (V2 records), 250 ms when idle; journal gap → rebuild | omni-search's concept. Create or rename-new upserts, delete removes, and an unknown FRN is added. |
| Live-update status at most 1/s; snapshot every 10 min while dirty and on `RunEvent::Exit` | A busy disk must not flood the UI or rewrite hundreds of MB per change |
| Folder walk runs on its own rayon pool | On the shared pool a walk starved every search (20 s observed) |

- **Measured (release):** a 1M-entry snapshot saves in about 110 ms and loads in about
  180 ms. On this machine C: + E: hold 6.3M entries and take 250 MB on disk.
- **Unverified here:** the live path needs an Administrator terminal:
  `cargo test --lib -- --ignored usn_sees_new_file --nocapture`.

## Consequences

- **UAC:** every release launch shows a UAC prompt, and child processes started by the
  app also run elevated.
- **Unverified here:** the MFT path cannot be checked from an unelevated shell. Run
  `cargo test --lib -- --ignored mft_enumerates_system_volume --nocapture` in an
  Administrator terminal.
- **`tauri dev` behaviour:** unelevated, every drive is walked, which is slower but
  correct.
