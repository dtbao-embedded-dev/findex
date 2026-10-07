---
title: Index Snapshot Format
category: data
order: 1
purpose: The FXI1 on-disk format of a drive's saved name index, and the in-memory VolumeIndex invariants it is rebuilt into.
status: active
updated: 2026-10-07
source: src-tauri/src/model/index/snapshot.rs, src-tauri/src/model/index/store.rs, src-tauri/src/model/index/mft.rs, src-tauri/src/model/index/walk.rs
confidence: confirmed
verified_at: 2026-10-07
keywords: fxi, FXI1, snapshot, fnv-1a, VolumeIndex, arena, frn, root_frn, journal, usn
---

# Index Snapshot Format

> One file per drive, `<app data>/index/<LETTER>.fxi`, rebuilt into a `VolumeIndex`.

## File layout (little-endian)

| Field | Type |
|-------|------|
| magic | `b"FXI1"` |
| version | u32 = 1 (any other is refused) |
| root | u32 length + UTF-8, e.g. `C:\` |
| root_frn | u64 |
| journal flag | u8: 0 = none (folder walk), 1 = present |
| journal | u64 id, i64 first_usn, i64 next_usn (only if flag = 1) |
| count | u64 live entries |
| entry × count | u64 frn, u64 parent frn, u8 is_dir, u32 length + UTF-8 name |
| trailer | u64 FNV-1a 64 of every byte before it |

- **Atomic writes:** the file is written to `<L>.fxi.tmp`, `sync_all`-ed, then renamed.
- **Bounds on load:** a string longer than 4096 bytes is corrupt. The reserve is
  capped by file size divided by 21 bytes, the minimum entry size.

## VolumeIndex invariants (`store.rs`)

- **Nodes:** each holds a parent FRN, not a parent index. Children may arrive before
  their parents, and a move touches one node.
- **Arenas:** original-case names live in one arena. Lowercase names live in another,
  each followed by `\0`.
- **`starts`:** lists `(lower offset, node)` in increasing offset order and is never
  rewritten. A rename leaves its old string as a dead entry. `owner_of` accepts a match
  only when the node's current lower offset equals that start.
- **`upsert`:** an unchanged entry is a no-op. Otherwise it appends new strings and
  counts the old ones as dead.
- **Deletes:** `remove` marks a node as a tombstone.
- **Compaction:** `compact_if_wasteful` rebuilds when dead bytes exceed half the
  arena + 1 MiB. Ids are renumbered; FRNs are not.
- **Root FRNs:** NTFS uses `ROOT_FRN = 0x0005_0000_0000_0005`. MFT records below 16,
  and children of metadata folders, are skipped. A walked drive uses root 0 and ids
  1, 2, 3, …
