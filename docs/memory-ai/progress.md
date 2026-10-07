---
title: Progress
updated: 2026-10-07
---

# Progress

> v0.1.0, unreleased, on `release/v0.1`: splash, fast name search on one active drive
> (snapshots, live USN updates), right-click actions with unelevated launches, file
> details, folder tags and auto update. Admin-only paths and a real update are not
> yet verified.

## What works

- **Splash window:** shows the icon, version and copyright. Main is revealed once it is
  ready, with a 10 s fallback. Checked by screenshot.
- **One active drive (2026-10-07, b5e17c3):** no setup screen; the first launch saves
  the system drive; the toolbar picker switches it. Checked over CDP: picking E: left
  only E: indexed and 200/200 hits under `E:\`.
- **Right-click actions (01c024b):** Open with Claude / folder / Terminal, Copy path /
  name, and Show in Explorer for files. Checked unelevated: Explorer, Windows Terminal
  and Claude Code opened in the folder; the clipboard read back.
- **File details (af7290b):** type, size and date per row, 200 rows in 2–32 ms after
  the search.
- **Folder tags (735321a):** editor, chips, `tag:` search, the tag list; a tag
  survived a restart.
- **Auto update (5782ca5):** a quiet check at launch + every 6 h; the tray check
  answered "Could not fetch a valid release JSON" (no release exists yet).
- **WebView menu blocked (63a3dea)** outside text fields.
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
- **Gates (e2bba88):** typecheck, vitest (75), vite build, cargo fmt, clippy
  `-D warnings` and cargo test (91, 6 ignored) all pass. The ignored launch tests
  (`a_process_started_as_the_shell_user…`, `a_program_in_a_spaced_path…`) pass here.
- **Second independent review:** 11 findings; 6 fixed in b036323..e2bba88, including
  a reproduced planted-`claude.cmd` hijack. The rest are listed in
  [actions, tags and updates](behavior/actions-tags-updates.md).
- **Independent review:** 7 findings, all fixed in 2a78413..b80de4f.

## What's left

1. **Human checks in an Administrator terminal** (not run; this shell is unelevated):
   - `cargo test --lib -- --ignored mft_enumerates_system_volume --nocapture`
   - `cargo test --lib -- --ignored usn_sees_new_file --nocapture`
   - Launch the release build: UAC appears, and C: is read via the MFT (no
     "Folder scan" note).
   - In that build, Open in Terminal → `whoami /groups | findstr Mandatory` shows
     **Medium** (the terminal is not elevated).
2. **First real update:** install a v0.1.0 build, publish v0.1.1, then check from the
   tray → banner → Restart now → v0.1.1 runs.
3. Content search (decide between FTS5 and a dedicated engine first).
4. Opening a result in any chosen app (only Explorer, terminal and Claude exist).

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
- The feature commits landed on `release/v0.1` by the user's choice (twice). CLAUDE.md
  reserves `release/*` for fixes.
- Tags are keyed by path: renaming or moving a tagged folder drops its tags.
- `claude` and `wt.exe` are looked up on the PATH Findex started with; installing
  either while it runs needs a restart of Findex.

## Verification

Name search evidence is in f18fbc1..aff652c; the 2026-10-07 second run's evidence is
the CDP checks and screenshots listed above (63a3dea..5782ca5). Earlier evidence:
- WebView2 DevTools runs (`cdp-search.mjs`, `cdp-early.mjs` in the session
  scratchpad).
- Screenshots.
- `--ignored` perf tests.
