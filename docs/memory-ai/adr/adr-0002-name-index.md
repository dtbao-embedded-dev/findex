---
title: "ADR-0002: Name Index"
category: architecture
purpose: How findex indexes file and folder names — MFT read as administrator, folder walk fallback, in-memory arena search.
status: active
updated: 2026-10-07
source: src-tauri/src/model/index/mft.rs, src-tauri/src/model/index/walk.rs, src-tauri/src/model/index/service.rs, src-tauri/src/model/index/store.rs, src-tauri/src/model/search.rs, src-tauri/src/model/win32.rs, src-tauri/build.rs, src-tauri/findex.manifest
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

## Consequences

- **UAC:** every release launch shows a UAC prompt, and child processes started by the
  app also run elevated.
- **Unverified here:** the MFT path cannot be checked from an unelevated shell. Run
  `cargo test --lib -- --ignored mft_enumerates_system_volume --nocapture` in an
  Administrator terminal.
- **`tauri dev` behaviour:** unelevated, every drive is walked, which is slower but
  correct.
