---
title: App Shell
category: architecture
order: 2
purpose: How the Tauri 2 + React 19 app is layered, which windows and screens exist, and which scaffold features are wired in.
status: active
updated: 2026-10-07
source: src-tauri/src/lib.rs, src-tauri/src/main.rs, src-tauri/tauri.conf.json, src-tauri/build.rs, src/main.tsx, src/splash.tsx, src/model/updater.ts, src/model/settings.ts, src/model/window.ts, src-tauri/src/tray.rs, src-tauri/src/splash.rs, src-tauri/src/model/db.rs, src-tauri/src/model/win32.rs, src-tauri/src/commands.rs, src-tauri/src/error.rs, src/view/App.tsx, src/view/TitleBar.tsx, src/view/SplashView.tsx, src/view/DrivePicker.tsx, src/view/ContextMenu.tsx, src/view/TagEditor.tsx, src/view/UpdateBanner.tsx, src/view/Toast.tsx, src/view/useDismiss.ts, src/view/blockWebMenu.ts, src/view/SearchView.tsx, src/view/StatusBar.tsx, src/styles.css, vite.config.ts, splash.html
confidence: confirmed
verified_at: 2026-10-07
keywords: tauri, react, pnpm, MVP, tray, single-instance, updater, sqlite, titlebar, AppResult, splash, search, drive picker, context menu, tags, apple-design, unsafe, win32
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

## Dependencies & build

- **Toolchain:** pnpm, Node 22, and the Rust toolchain pinned in `rust-toolchain.toml`
  (keep it in step with both workflows).
- **Vite:** builds two pages (`index.html`, `splash.html`).
- **Rust crates:** `rayon`, `memchr`, and `windows-sys 0.61` (Win32 Foundation,
  Security, Storage_FileSystem, System_Com, System_IO, System_Ioctl,
  System_Diagnostics_Debug, System_Threading, System_WindowsProgramming, UI_Shell,
  UI_WindowsAndMessaging).
- **Target and CI:** Windows-only NSIS bundle; CI on `windows-latest`. Dev server port
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
