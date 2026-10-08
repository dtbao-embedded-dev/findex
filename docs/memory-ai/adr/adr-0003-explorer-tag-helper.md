---
title: "ADR-0003: Explorer Tag Helper"
category: architecture
purpose: Why quick tagging from Explorer uses a separate unelevated helper crate and an inbox folder rather than findex.exe itself.
status: active
updated: 2026-10-07
source: src-tauri/build.rs, src-tauri/Cargo.toml, src-tauri/tag-helper/src/main.rs, src-tauri/src/model/inbox.rs, src-tauri/tauri.release.conf.json
confidence: confirmed
verified_at: 2026-10-07
keywords: findex-tag, requireAdministrator, embed_resource, externalBin, inbox, asInvoker, quick tag
---

# ADR-0003: Explorer Tag Helper

> Tagging a folder "without opening the app" runs a small unelevated helper from
> Explorer's menu; the helper only drops a request file that Findex applies.

## Context

The user asked for a way to mark a folder while Findex is not open (2026-10-07) and
chose Explorer's right-click menu over a global hotkey or a tray item.
[ADR-0002](adr-0002-name-index.md) keeps `requireAdministrator` in the release
manifest, so any launch of `findex.exe` prompts UAC.

## Decision

| Choice | Why |
|--------|-----|
| Separate crate `src-tauri/tag-helper` (workspace member) | `tauri-build` links its manifest resource into every bin of the `findex` package (verified in tauri-build 2.7.1 / tauri-winres `embed_resource::compile`), so a second `[[bin]]` would also require admin. User chose this over dropping the manifest for a runtime relaunch. |
| Helper never opens SQLite; writes `<inbox>/<ms>-<pid>.req` (tmp then rename) | No database dependency, no write race with the app; requests survive while Findex is not running. |
| Findex polls the inbox every second | Simple and cheap; no IPC across integrity levels (UIPI). |
| `externalBin` only in the release config | tauri-build requires the file to exist; dev builds the helper into `target/debug` instead. |

## Consequences

- A release needs Python for `docs/scripts/build-helper.py` (`beforeBuildCommand`).
- Tag requests are applied only while, or the next time, Findex runs.
- See [Explorer quick tag](../behavior/explorer-quick-tag.md).
