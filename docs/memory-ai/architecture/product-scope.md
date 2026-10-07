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

Stated by the user on 2026-10-07 when the repository was scaffolded. None of these
capabilities exists in code yet; the scaffold only carries a greeting example and a
settings table.

| Capability | What the user asked for |
|------------|-------------------------|
| Name search | Fast search over file and folder names |
| Content search | "Advanced" keyword search that also matches text inside files |
| Folder tags | Attach tags to folders and find folders by tag |
| Quick open | Open a result in a terminal, or in any chosen application |

## Reproduction notes

- Tags and the content index must persist across restarts; this is why the `sqlite`
  feature was chosen (see [adr-0001](../adr/adr-0001-scaffold-choices.md)).
- The full-text index approach (SQLite FTS5 vs a dedicated engine) is open and
  unresolved until decided.
- Quick open will need Tauri capabilities (shell/opener) that the scaffold does not
  grant; `capabilities/default.json` must be widened deliberately.

## See also

- [App shell](app-shell.md)
