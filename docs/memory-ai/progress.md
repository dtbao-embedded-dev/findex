---
title: Progress
updated: 2026-10-07
---

# Progress

> v0.1.0, unreleased: the splash, drive setup and fast name search with snapshots and
> live USN updates are built on `release/v0.1`. Admin-only paths are not yet verified.

## What works

- **Splash window:** shows the icon, version and copyright. Main is revealed once it is
  ready, with a 10 s fallback. Checked by screenshot.
- **First-run drive setup:** stores the choice in `scan.drives`; the Drives button
  reopens it. Checked by screenshots and a read of the setting.
- **Name index:**
  - **Reading drives:** MFT for NTFS when elevated, otherwise a parallel folder walk on
    its own pool.
  - **Snapshots:** per-drive `.fxi` files, loaded before the rebuild.
  - **Live updates:** a USN watcher on NTFS drives.
- **Search screen:** live results, latest-query-wins, ↑/↓/Enter/Esc, Explorer reveal,
  and a status bar.
- **Numbers, measured 2026-10-07:**
  - **Search, 1M synthetic names (release):** p95 ≤ 15.3 ms.
  - **Snapshot, 1M entries (release):** load ~180 ms, save ~110 ms.
  - **This machine (debug, unelevated walk):** 6.28M entries on C: + E:. A query is
    ~30 ms end to end. The snapshot is searchable at about 7 s of a 34 s rebuild.
- **Gates:** typecheck, vitest (39), vite build, cargo fmt, clippy `-D warnings` and
  cargo test (60) all pass.
- **Independent review:** 7 findings, all fixed in 2a78413..b80de4f.

## What's left

1. **Human checks in an Administrator terminal** (not run; this shell is unelevated):
   - `cargo test --lib -- --ignored mft_enumerates_system_volume --nocapture`
   - `cargo test --lib -- --ignored usn_sees_new_file --nocapture`
   - Launch the release build: UAC appears, and C: is read via the MFT (no
     "Folder scan" note).
2. Content search (decide between FTS5 and a dedicated engine first).
3. Folder tags (needs a new migration).
4. Quick open in a terminal or any app. Because the app runs elevated, children
   inherit admin rights.

## Known issues

- `main` branch protection must be enabled on GitHub by hand.
- **Unverified journal case:** a journal deleted and recreated may make
  `FSCTL_READ_USN_JOURNAL` return `ERROR_INVALID_PARAMETER`, which `is_journal_gap`
  does not treat as a gap. The status would then say "Live updates stopped" instead
  of rebuilding. Confirming it needs an Administrator test.
- Snapshots are large: about 40 B per entry, 250 MB for C: + E:. A compact encoding
  (varint, delta-encoded FRNs) would shrink them.
- The folder walk peaks at about 1.1 GB, because it collects all entries before
  inserting them.
- Results include `$Recycle.Bin` contents, as Everything's do; whether to filter them
  is undecided.
- The feature commits landed on `release/v0.1` by the user's choice. CLAUDE.md
  reserves `release/*` for fixes.

## Verification

Evidence is in the T1–T7 commits on `release/v0.1` (f18fbc1..aff652c):
- WebView2 DevTools runs (`cdp-search.mjs`, `cdp-early.mjs` in the session
  scratchpad).
- Screenshots.
- `--ignored` perf tests.
