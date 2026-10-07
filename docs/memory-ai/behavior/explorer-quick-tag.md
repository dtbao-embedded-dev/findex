---
title: Explorer Quick Tag
category: behavior
order: 3
purpose: How a folder is tagged from Explorer's right-click menu while Findex stays in the tray — the findex-tag helper, the inbox folder, the registry verbs and their lifecycle.
status: active
updated: 2026-10-07
source: src-tauri/tag-helper/src/main.rs, src-tauri/tag-helper/Cargo.toml, src-tauri/src/model/inbox.rs, src-tauri/src/model/shell_menu.rs, src-tauri/src/model/win32.rs, src-tauri/src/commands.rs, src-tauri/src/lib.rs, src-tauri/windows/hooks.nsh, src-tauri/tauri.conf.json, src-tauri/tauri.release.conf.json, src-tauri/Cargo.toml, docs/scripts/build-helper.py, package.json, src/model/explorer.ts, src/presenter/explorerPresenter.ts, src/view/ExplorerSettings.tsx, src/presenter/tagPresenter.ts, src/view/App.tsx
confidence: confirmed
verified_at: 2026-10-07
keywords: findex-tag, inbox, .req, tags-changed, shell_menu, FindexTag, MUIVerb, SubCommands, %V, explorer.menu, explorer_menu, set_explorer_menu, sync_explorer_menu, externalBin, installerHooks, NSIS_HOOK_PREUNINSTALL, RegCreateKeyExW, RegDeleteTreeW, watch_inbox
---

# Explorer Quick Tag

> Explorer's "Tag with Findex ▸ <tag>" runs the unelevated `findex-tag.exe`, which
> drops a request file into `<app data>/inbox`; the running Findex applies it within
> a second (or at its next start) and tells the window to read the tags again.

## Why a helper

Findex runs as administrator, and `tauri-build` links the `requireAdministrator`
manifest into every binary of the `findex` crate (`embed_resource` link-arg-bins),
so a second `[[bin]]` would prompt UAC on every click. The helper is its own crate,
`src-tauri/tag-helper` (workspace member, std only, `windows_subsystem = "windows"`,
no manifest → asInvoker). See [ADR-0003](../adr/adr-0003-explorer-tag-helper.md).

## Flow

```mermaid
sequenceDiagram
  Explorer->>findex-tag: --inbox "<dir>" --tag working "%V"
  findex-tag->>inbox: write <ms>-<pid>.tmp, rename to .req ("tag\nfolder\n")
  Findex(watch_inbox, 1 s)->>inbox: inbox::drain
  Findex->>DB: tags::add(folder, tag)
  Findex->>Window: event tags-changed → watchTags → catalog + row chips refresh
```

- **Helper (`main.rs`):** parses the options in any order, validates the tag with the
  same rule as `tags::normalize_tag`, refuses a folder with CR/LF. A trailing `"` on
  the folder is read back as `\` (a drive root `"E:\"` arrives as `E:"`, 6683abb).
  Exit code only; no UI.
- **Drain (`inbox.rs`):** skips an inbox that is not a real directory, files that are
  links or larger than 4 KiB, and `.tmp` files; a request that cannot be applied (bad
  tag, missing folder) is deleted, not retried. Returns how many tagged a folder.
- **Watcher (`commands::watch_inbox`):** a thread started in setup, drains every
  second, emits `tags-changed` when > 0.

## Registry menu (`shell_menu.rs`)

- **Roots:** `HKCU\Software\Classes\Directory\shell\FindexTag` and
  `...\Directory\Background\shell\FindexTag` (`MUIVerb` "Tag with Findex", `Icon`
  findex.exe,0, `SubCommands` ""). Entries `shell\01`..`16` in catalog order (presets
  capitalised, custom as `#tag`), each `command` = `"<helper>" --inbox "<inbox>"
  --tag <tag> "%V"`. Windows 11 shows it under "Show more options".
- **`sync`:** deletes both roots, then writes them again if setting `explorer.menu`
  is not `"off"` (on by default) and the helper exists; else an error naming it.
- **When:** at setup (`commands::sync_menu`), after a tag-name change (App effect on
  `menuTagsKey`), and when Settings → Explorer opens or toggles.
- **Paths:** helper = `current_exe` dir + `findex-tag.exe` (installer: `externalBin`;
  dev: `pnpm run helper:dev` builds it into `target/debug`).
- **Uninstall:** `windows/hooks.nsh` `NSIS_HOOK_PREUNINSTALL` deletes both keys.

## Build

- `tauri.release.conf.json` adds `bundle.externalBin: ["binaries/findex-tag"]`;
  `beforeBuildCommand` runs `pnpm run helper` (`docs/scripts/build-helper.py`: release
  build, copy to `src-tauri/binaries/findex-tag-<host triple>.exe`, gitignored).
- `externalBin` is kept out of the main config because tauri-build fails when the file
  is missing, and `tauri dev` starts cargo before `beforeDevCommand` could build it.
- CI runs fmt `--all`, clippy and test `--workspace`.

## Limits

- Elevating with a different admin account writes that account's HKCU.
- Add-only: the menu cannot remove a tag.
- Not yet checked by a human in Explorer itself; the registered command lines were run
  as Explorer would and tagged a folder and `E:\` (2026-10-07).

## See also

- [Actions, tags and updates](actions-tags-updates.md)
- [Commands and events](../interface/commands-and-events.md)
