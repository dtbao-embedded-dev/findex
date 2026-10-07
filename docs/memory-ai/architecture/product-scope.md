---
title: Product Scope
category: architecture
order: 1
purpose: What findex is for — the four user-facing capabilities the app is being built toward.
status: active
updated: 2026-10-07
source: conversation
confidence: confirmed
verified_at: 2026-10-07
keywords: findex, file search, content search, folder tags, quick open, launcher
---

# Product Scope

> findex is a Windows desktop launcher that finds files and folders fast, searches inside
> file contents, tags folders, and opens a result in a terminal or any app.

## Responsibility

Stated by the user on 2026-10-07 when the repository was scaffolded. Name search is
built (see [ADR-0002](../adr/adr-0002-name-index.md)). Folder tags and quick open
(Explorer, terminal, Claude Code, copy path or name) were built later the same day (see
[actions, tags and updates](../behavior/actions-tags-updates.md)). Content search was
built on branch `feat/content-search-paging` (2026-10-07) as a direct, streamed scan
with no index (see [search and indexing](../behavior/search-and-indexing.md)).
Opening in any chosen app is not built yet.

| Capability | What the user asked for |
|------------|-------------------------|
| Name search | Fast search over file and folder names |
| Content search | "Advanced" keyword search that also matches text inside files |
| Folder tags | Attach tags to folders and find folders by tag |
| Quick open | Open a result in a terminal, or in any chosen application |

## Reproduction notes

- Tags must persist across restarts; this is why the `sqlite` feature was chosen (see
  [adr-0001](../adr/adr-0001-scaffold-choices.md)).
- Content search engine, decided by the user on 2026-10-07: scan files on demand and
  stream results (no FTS5, no dedicated engine). Rationale: works at once with nothing
  to build or keep current; cost: a whole-drive scan takes minutes on a cold disk, so
  the Type filter and tags are the way to narrow it. A persistent index stays possible
  later if scans prove too slow.
- Quick open needed no new Tauri capability: launches go through Rust commands
  (`launch.rs`), so `capabilities/default.json` was not widened.

## See also

- [App shell](app-shell.md)
