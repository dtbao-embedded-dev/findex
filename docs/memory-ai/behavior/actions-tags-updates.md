---
title: Actions, Tags and Updates
category: behavior
order: 2
purpose: What the right-click actions, file details, folder tags and the auto-update flow do, and the rules behind each.
status: active
updated: 2026-10-07
source: src-tauri/src/model/launch.rs, src-tauri/src/model/win32.rs, src-tauri/src/model/details.rs, src-tauri/src/model/tags.rs, src-tauri/src/model/find.rs, src-tauri/src/model/db.rs, src-tauri/src/tray.rs, src/presenter/menuPresenter.ts, src/presenter/tagPresenter.ts, src/presenter/updatePresenter.ts, src/presenter/searchPresenter.ts, src/model/updater.ts, src/view/blockWebMenu.ts, src/view/SearchView.tsx
confidence: confirmed
verified_at: 2026-10-07
keywords: context menu, open_claude, open_terminal, create_process, de-elevate, PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, wt.exe, details, SHGetFileInfoW, folder_tag, tag:, updater, save_index, check-updates, blockWebMenu
---

# Actions, Tags and Updates

> Built on 2026-10-07 (commits 63a3dea..e2bba88 on `release/v0.1`). See
> [commands and events](../interface/commands-and-events.md) for the exact contract.

## Right-click actions (`menuPresenter`, `launch.rs`)

- **Menus:** a folder gets Open with Claude, Open folder, Open in Terminal, Copy path,
  Copy name, Tags…; a file gets Show in Explorer, Copy path, Copy name. The menu key or
  Shift+F10 opens it for the selected row; the field's own menu is then suppressed.
- **Normal rights:** Findex runs elevated, so `win32::create_process(cmd, dir, true)`
  makes Explorer the parent (`GetShellWindow` → PID → `OpenProcess(CREATE_PROCESS)` →
  `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS`); the child takes Explorer's token. Elevated
  and failing → refused with an error, never started elevated. Unelevated and failing →
  a plain `CreateProcessW`.
- **No folder on the command line:** the folder is the working directory. Terminal =
  `"<wt.exe>" -d .` (found on PATH, app execution aliases included), else
  `powershell.exe -NoExit`.
- **Claude, folder untrusted (b036323):** the `claude` found on PATH runs by its full
  path, never by name: `cmd /k claude` had looked in the folder first, so a planted
  `claude.cmd` ran instead (reproduced, then fixed). WT gets `-d . "<path>"` (it
  re-parses and re-quotes its arguments, which mangled `""…""`); without WT,
  `cmd.exe /s /k ""<path>""`. The env block adds `NoDefaultCurrentDirectoryInExePath=1`.
- **Elevation unknown = elevated:** `win32::elevation()` returns `None` when the token
  cannot be read; `start_with` then refuses instead of a plain start.
- **Open folder:** `explorer.exe "<dir>"`; a root gets `"C:\."` so the closing quote
  is not escaped.
- **Copy:** `navigator.clipboard.writeText`; a "Path copied" / "Name copied" toast.
- **WebView menu:** `blockWebMenu` cancels `contextmenu` unless the target is a text
  input, textarea or contenteditable.

## File details (`details.rs`, `latestDetails`)

- Read after the hits arrive, with `std::fs::metadata` in parallel; vanished paths are
  omitted. Type names come from `SHGetFileInfoW(SHGFI_TYPENAME | USEFILEATTRIBUTES)`,
  cached per extension (COM initialised per call). Fallback: "XYZ File", "File folder".
- Measured on this machine (debug): 200 rows take 2–32 ms after a 25–46 ms search.

## Folder tags (`tags.rs`, `find.rs`)

- **Tag:** trimmed, leading `#` dropped, lower case, 1–32 letters, digits, `-`, `_`.
- **Key:** the folder path without a trailing backslash (roots keep it), compared
  without case (`COLLATE NOCASE`, ASCII only). Rename or move → tags are lost.
- **Search:** `tag:` queries never reach the index; `find::search` reads folders with
  every tag, skips missing ones, ranks by name (`rank_name`), then by name.
- **UI:** chips on rows (from `describe_paths`), the editor saves each change at once,
  the empty screen lists tags with counts.

## Auto update (`updatePresenter`, `model/updater.ts`)

- First check 10 s after mount, then every 6 h. Found → `update.download()` → banner
  "Restart now / Later". Later hides it; the download is kept and offered at the next
  check (no second download).
- A background check that finds nothing or fails shows nothing. The tray's Check for
  updates (event `check-updates`) shows "Checking…", then "up to date" or the error.
- Restart: `save_index` first (the NSIS installer exits via `std::process::exit`,
  skipping the `RunEvent::Exit` save), then `install()`. A failed save still installs.

## Independent review (2026-10-07)

11 findings. Fixed: planted `claude.cmd` (b036323), unknown elevation failing open
(b036323), duplicate `tag:` names matching nothing (9ee21a0), a tray check during a
background check answering nothing and a double Restart (b3e5637), the menu acting
on a reordered row (e2bba88). Left open, all LOW:
- **Environment:** the child gets Findex's environment, not Explorer's
  (`CreateEnvironmentBlock` not used). An already-running Windows Terminal launches
  with its own environment, so `NoDefaultCurrentDirectoryInExePath` only protects the
  cmd fallback; an npm `claude.cmd` shim calling bare `node` could still find a
  `node.exe` in the folder under WT (suspected, not tested; this machine's Claude is
  a native `claude.exe`).
- **Case:** `COLLATE NOCASE` folds ASCII only; non-ASCII case variants of one path
  become two keys.
- **Races:** two drive picks in quick succession may save one and index the other
  (suspected). `describe` waits on the disk inside the global rayon pool that search
  also uses (suspected).
- **Explorer elevated** (UAC off, built-in Administrator): children inherit that token.

## Not verified

- **Elevated launch:** the de-elevated path was exercised only unelevated. Needs an
  Administrator run: Open in Terminal → `whoami /groups | findstr Mandatory` → Medium.
- **A real update:** no release exists yet; the endpoint answers "Could not fetch a
  valid release JSON". Needs v0.1.0 installed, then v0.1.1 published.
- **Tray click:** the frontend listener was tested by emitting `check-updates`; the
  tray menu item itself was not clicked.
