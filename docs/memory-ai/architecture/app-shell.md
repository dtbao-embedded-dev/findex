---
title: App Shell
category: architecture
order: 2
purpose: How the Tauri 2 + React 19 app is layered, which windows and screens exist, and which scaffold features are wired in.
status: active
updated: 2026-10-07
source: src-tauri/src/lib.rs, src-tauri/src/main.rs, src-tauri/tauri.conf.json, src-tauri/build.rs, src/main.tsx, src/splash.tsx, src/model/updater.ts, src/model/settings.ts, src/model/window.ts, src-tauri/src/tray.rs, src-tauri/src/splash.rs, src-tauri/src/model/db.rs, src-tauri/src/model/win32.rs, src-tauri/src/commands.rs, src-tauri/src/error.rs, src/view/App.tsx, src/view/TitleBar.tsx, src/view/SplashView.tsx, src/view/DrivePicker.tsx, src/view/ContextMenu.tsx, src/view/TagEditor.tsx, src/view/UpdateBanner.tsx, src/view/Toast.tsx, src/view/useDismiss.ts, src/view/blockWebMenu.ts, src/view/SearchView.tsx, src/view/StatusBar.tsx, src/view/FilterBar.tsx, src/view/HistoryMenu.tsx, src/view/AppearanceSettings.tsx, src/view/ExplorerSettings.tsx, src/presenter/themePresenter.ts, src/presenter/historyPresenter.ts, src/presenter/updatePresenter.ts, src/presenter/drivePresenter.ts, src/view/ResultList.tsx, src/view/SettingsSheet.tsx, src/view/TagSettings.tsx, src/view/TagMark.tsx, src/presenter/aboutPresenter.ts, src/presenter/listPresenter.ts, src/presenter/contentPresenter.ts, src/presenter/filterPresenter.ts, src/styles.css, vite.config.ts, splash.html
confidence: confirmed
verified_at: 2026-10-07
keywords: tauri, react, pnpm, MVP, search row, data-theme, appearance.theme, dark only, search.history, back-to-top, update button, status-copyright, workspace, filter bar, settings sheet, virtual list, re-scan, tray, single-instance, updater, sqlite, titlebar, AppResult, splash, search, drive picker, context menu, tags, apple-design, unsafe, win32
---

# App Shell

> A Tauri 2 desktop app with a React 19 MVP frontend and a Rust backend whose commands
> stay thin and delegate to `model/`. Two windows (splash, main); main opens straight on
> search.

## Layout

| Path | Holds |
|------|-------|
| `src/model/` | The only frontend code that names a Tauri command, event or plugin |
| `src/presenter/` | Logic taking model calls as parameters, so tests need no Tauri runtime |
| `src/view/` | React components; `App.tsx` renders search, the update banner and the status bar |
| `src-tauri/src/lib.rs` | Builder chain: plugins, setup (db, `IndexService`), window events, command table, `RunEvent::Exit` save |
| `src-tauri/src/commands.rs` | `#[tauri::command]` wrappers, each calling one `model/` function; `status_emitter` |
| `src-tauri/src/model/` | Business rules; `db.rs` owns SQLite, `index/` the name index, `search.rs` the query engine |
| `src-tauri/src/model/win32.rs` | The only module allowed `unsafe` (`#![allow(unsafe_code)]`); every block has a `SAFETY:` comment |
| `src-tauri/src/splash.rs`, `tray.rs` | Window plumbing outside `model/`, like the tray |
| `src-tauri/src/error.rs` | `AppError` / `AppResult<T>`, serialized as the message string |

## Windows and screens

- **splash** (`splash.html`, `SplashView`): 360×280, no decorations, skipTaskbar.
  - Shows the icon, name, `Version x.y.z` and copyright.
  - Main (`visible: false`) calls `splash_ready` once routed, after at least 900 ms.
    `splash::finish` runs once (static `AtomicBool`) and a 10 s Rust fallback shows
    main anyway.
- **search** (`SearchView` + `StatusBar`): a search field with live results. ↑/↓ move
  the selection, Enter or a double click reveals the result in Explorer, Esc clears.
  - **Drive picker** (`DrivePicker`): a toolbar button naming the active drive; its
    menu lists every drive and calls `set_active_drive`. It replaced the setup screen
    on 2026-10-07.
  - **Rows:** name and tag chips over the folder; type · size over the modified date
    on the right (`describe_paths`, fetched after the hits).
  - **Context menu** (`ContextMenu`): right-click, the menu key or Shift+F10; items
    from `menuPresenter.menuFor`. The **tag editor** (`TagEditor`) opens from Tags….
  - **Toast** (`Toast`): copy confirmations and action errors over the status bar.
- **update banner** (`UpdateBanner`): a card above the status bar for a downloaded
  update (Restart now / Later) or a tray check's answer.
- **WebView menu:** `blockWebMenu` (installed in `main.tsx` and `splash.tsx`) cancels
  `contextmenu` everywhere except text fields.

## Wired features

- **tray**: single-instance plugin registered first; a second launch shows the running
  window. `CloseRequested` on `main` hides it; only tray Quit exits.
- **updater**: updater and process plugins; endpoint is the GitHub `latest.json` of
  `dtbao-embedded-dev/findex` (see [updater signing key](../rule/updater-signing-key.md)).
  The flow runs in the frontend (`updatePresenter`); see
  [actions, tags and updates](../behavior/actions-tags-updates.md). The tray menu is
  Show, Check for updates, Quit.
- **sqlite**: `<app data dir>/findex.sqlite3`, managed as `Db`; tables `setting` and
  `folder_tag` (migration 2).
- **titlebar**: `decorations: false`; custom bar with a drag region and four window
  permissions.
- **elevation**: `build.rs` embeds `findex.manifest` (`requireAdministrator`) only for
  `PROFILE=release`. It also passes `bundle.copyright` as `FINDEX_COPYRIGHT`, because
  tauri's generated runtime config drops that field.

## UI design

The UI follows the `apple-design` skill; the tokens live on `:root` in `src/styles.css`:
- **Type:** system Segoe UI Variable.
- **Materials:** translucent `--material` with `backdrop-filter`.
- **Press feedback:** `:active scale(.97)`.
- **Motion:** critically damped `--ease`.
- **Accessibility:** `prefers-reduced-motion`, `prefers-reduced-transparency` and
  `prefers-contrast` are all handled.

## Main window after the redesign (branch `feat/ui-redesign-filters`, 2026-10-07)

- **Title bar** (`TitleBar.tsx`): icon, name and `v<version>` from `app_about`
  (`aboutPresenter.loadTitle`); a re-scan button (spins and is disabled while
  `status.busy`) and a Settings gear left of the window buttons.
- **Workspace** (`SearchView.tsx`): a grid of `Sidebar` (Location: `DrivePicker` as a
  row; Tags: the catalog, presets as coloured dots, a press toggles the tag filter)
  and the search pane: field, `FilterBar` (Name/Contents segmented control, date
  popover, All/Files/Folders segmented control, Type popover, tag popover, Clear
  filters — which keeps the mode) and `ResultList`.
- **`ResultList.tsx`:** virtualized; fixed-height rows placed by index
  (`listPresenter.visibleRange`, `scrollToShow`), details read only for the rows in
  view in chunks of 50 (`detailsWindow`). `rows` ref keeps the rendered row elements
  by index for the keyboard-opened menu. A row with `line` (a content match) shows
  the line number and the snippet in place of the folder, which moves to its tooltip.
  Double-click does nothing.
- **`SettingsSheet.tsx`:** a modal sheet under the title bar (scrim, Esc, Tab trap,
  focus returns to the opener) holding `TagSettings` (the preset tags as pills; create
  and delete custom tags, delete confirmed inline). The results-limit section was
  removed in d30ce23.
- **State held by `App`:** the tag catalog, shared by the sheet, the sidebar, the
  filters and the tag editor (`refreshTags`).
- **Superseded (branch `feat/toolbar-themes-quick-tag`, 8c10406):** the sidebar and
  the title-bar re-scan button are gone; see the next section.
- **Stacking:** the sidebar and the search header carry `z-index`, because their
  `backdrop-filter` / mask make stacking contexts that otherwise let the results paint
  over the drive menu and the filter popovers (found by screenshot, fixed in T1).

## Main window after branch `feat/toolbar-themes-quick-tag` (2026-10-07)

- **Search row** (`SearchView.tsx` `.search-row`): compact `DrivePicker` (glyph +
  `driveLetterLabel`, e.g. `E:`; full name and free space in the tooltip), the field
  (now a `div.search-field`, holding `HistoryMenu`), then the re-scan `.search-tool`.
  `Sidebar.tsx` was deleted; tags filter through the FilterBar Tags menu and chips.
- **Title bar tools:** update button (`updatePresenter.updateButton`: check / checking /
  accent badge while an update is `ready` or `deferred`), then Settings.
- **Settings sheet sections:** Appearance (`AppearanceSettings`, six swatches), Tags,
  Explorer (`ExplorerSettings` switch, see [Explorer quick tag](../behavior/explorer-quick-tag.md)).
- **Status bar:** ends with `title.copyright` (`.status-copyright`).
- **Results:** folder sizes and the back-to-top button, see
  [search extras](../behavior/search-extras.md).
- **Themes:** dark only (`color-scheme: dark`, light palette removed). Graphite is the
  `:root, [data-theme="graphite"]` default; Midnight, Aurora, Violet, Forest, Ember
  override surfaces/accent under `[data-theme]`; `App` sets `<html data-theme>` from
  setting `appearance.theme` before `revealMain`; the splash applies it too. Light
  accents (Aurora, Forest, Ember) use a near-black `--on-accent`. The contrast and
  transparency media blocks come after the themes so they still win.

## Dependencies & build

- **Toolchain:** pnpm, Node 22, and the Rust toolchain pinned in `rust-toolchain.toml`
  (keep it in step with both workflows).
- **Vite:** builds two pages (`index.html`, `splash.html`).
- **Rust crates:** `rayon`, `memchr`, and `windows-sys 0.61` (Win32 Foundation,
  Security, Storage_FileSystem, System_Com, System_IO, System_Ioctl,
  System_Diagnostics_Debug, System_Registry, System_Threading, System_WindowsProgramming, UI_Shell,
  UI_WindowsAndMessaging).
- **Workspace:** `src-tauri/Cargo.toml` is also a workspace root with member
  `tag-helper` (see [ADR-0003](../adr/adr-0003-explorer-tag-helper.md)).
- **Target and CI:** Windows-only NSIS bundle (uninstall hook `windows/hooks.nsh`); CI on `windows-latest`. Dev server port
  1832.
- **Release-mode tests:** run them with `--lib`. The release app binary may require
  elevation.

## Naming and icon

- **Display name:** **Findex** (`productName`, window title, tray tooltip, release
  title).
- **Machine names:** lowercase `findex` for the npm package, crate, bundle id
  `com.dtbao.findex` and repository.
- **Icons:** generated from `src-tauri/icons/icon.svg`. The splash imports that SVG
  directly.

## See also

- [Product scope](product-scope.md)
- [ADR-0002: name index](../adr/adr-0002-name-index.md)
- [Commands and events](../interface/commands-and-events.md)
- [Database migrations rule](../rule/database-migrations.md)
