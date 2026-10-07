---
title: Search Extras
category: behavior
order: 4
purpose: The behaviour added around search on 2026-10-07 — folder sizes, recent searches, the back-to-top button and Open with VS Code.
status: active
updated: 2026-10-07
source: src-tauri/src/model/folder_size.rs, src-tauri/src/commands.rs, src-tauri/src/model/launch.rs, src/model/details.ts, src/presenter/searchPresenter.ts, src/presenter/historyPresenter.ts, src/presenter/listPresenter.ts, src/presenter/menuPresenter.ts, src/view/SearchView.tsx, src/view/ResultList.tsx, src/view/HistoryMenu.tsx
confidence: confirmed
verified_at: 2026-10-07
keywords: folder_sizes, Sizer, size_of, latestSizes, describeHit, FolderBytes, Calculating, remember, forget, parseHistory, REMEMBER_AFTER_MS, HISTORY_MAX, HistoryMenu, showsBackToTop, back-to-top, open_vscode, find_vscode, vscode_target
---

# Search Extras

> Folder rows show a measured size, the empty field offers recent searches, a button
> returns a long list to the top, and VS Code opens folders and files.

## Folder sizes (76f3831)

- **Rust:** `folder_size::Sizer` (managed state) owns a 4-thread rayon pool, a
  generation counter and a 5-minute cache keyed by lower-cased path. `sizes` bumps the
  generation (stopping older walks), sends cached sizes at once, then walks the rest in
  parallel and sends each as it ends. `size_of` sums `DirEntry::metadata().len()`,
  never follows `is_symlink()` entries (junctions included), counts unreadable
  subfolders as 0, and gives `None` for an unreadable root or a cancelled walk.
- **Frontend:** `SearchView` asks `latestSizes` for the visible folder rows not yet
  known (read through a ref so an arriving size does not re-ask and cancel); when the
  latest request ended, folders without an answer are stored as `null`. Sizes reset
  only once a new query's own hits arrive, so old rows never flash "Calculating…".
- **Wording:** `describeHit(details, folderSize)` → `File folder · 2.4 GB`,
  `· Calculating…` while `"pending"`, no size for `null`.
- Checked: `docs/memory-ai` = 80 117 bytes, equal to PowerShell `Measure-Object`.

## Recent searches (4c4fa9e)

- `historyPresenter.remember`: trim, ignore < 2 chars, newest first, case-insensitive
  de-dup, a query extending the newest entry replaces it, cap 20; same array returned
  when unchanged (so nothing is saved). Saved as setting `search.history`.
- Recorded after `REMEMBER_AFTER_MS` (1.5 s) on screen with results, or when a result's
  menu action is picked.
- `HistoryMenu` shows while the field is focused, empty and not dismissed; mouse-down
  is prevented so focus stays in the field. ↑/↓/Enter pick, Esc dismisses (until blur
  or typing), × forgets one, Clear empties.

## Back to the top (0bcf7d2)

`listPresenter.showsBackToTop(scrollTop, height)` = scrolled past one viewport;
`ResultList` renders a round translucent button that `scrollTo`s 0 (smooth, or instant
under reduced motion).

## Open with VS Code (443e803)

Menu item for folders (after Open with Claude) and files (first). See
`open_vscode` in [commands and events](../interface/commands-and-events.md). Checked:
a folder opened a new window, a file a tab in the existing window.

## See also

- [Search and indexing](search-and-indexing.md)
- [Actions, tags and updates](actions-tags-updates.md)
