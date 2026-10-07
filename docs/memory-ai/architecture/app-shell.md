---
title: App Shell
category: architecture
order: 2
purpose: How the Tauri 2 + React 19 app is layered and which scaffold features are wired in.
status: active
updated: 2026-10-07
source: src-tauri/src/lib.rs, src-tauri/tauri.conf.json, src-tauri/src/tray.rs, src-tauri/src/model/db.rs, src-tauri/src/commands.rs, src-tauri/src/error.rs, src/view/App.tsx, src/view/TitleBar.tsx, src/model/updater.ts, src/model/settings.ts, src/model/window.ts
confidence: confirmed
verified_at: 2026-10-07
keywords: tauri, react, pnpm, MVP, tray, single-instance, updater, sqlite, titlebar, AppResult
---

# App Shell

> A Tauri 2 desktop app with a React 19 MVP frontend and a Rust backend whose commands
> stay thin and delegate to `model/`; tray, updater, sqlite and titlebar are wired in.

## Layout

| Path | Holds |
|------|-------|
| `src/model/` | The only frontend code that names a Tauri command or plugin |
| `src/presenter/` | Logic taking model calls as parameters, so tests need no Tauri runtime |
| `src/view/` | React components; `TitleBar.tsx` receives window actions as callbacks |
| `src-tauri/src/lib.rs` | Builder chain: plugins, setup, window events, command table; no business rules |
| `src-tauri/src/commands.rs` | `#[tauri::command]` wrappers, each calling one `model/` function |
| `src-tauri/src/model/` | Business rules; `db.rs` owns the SQLite connection |
| `src-tauri/src/error.rs` | `AppError` / `AppResult<T>`, serialized as the message string |
| `src-tauri/src/tray.rs` | Tray icon with Show and Quit |

## Wired features

- **tray**: single-instance plugin registered first; a second launch shows the running
  window. `CloseRequested` hides the window; only tray Quit exits.
- **updater**: updater and process plugins; the endpoint is the GitHub `latest.json` of
  `dtbao-embedded-dev/findex`. `pubkey` is still a placeholder (see progress).
- **sqlite**: `model::db::open` creates `<app data dir>/findex.sqlite3` during setup and
  it is managed as `Db` state; the first migration creates the `setting` table.
- **titlebar**: `decorations: false`; custom bar with a drag region and four window
  permissions.

## Dependencies & build

pnpm, Node 22, Rust toolchain pinned in `rust-toolchain.toml` (keep in step with both
workflows). Windows-only NSIS bundle; CI on `windows-latest`. Dev server port 1832.

## See also

- [Product scope](product-scope.md)
- [Database migrations rule](../rule/database-migrations.md)
