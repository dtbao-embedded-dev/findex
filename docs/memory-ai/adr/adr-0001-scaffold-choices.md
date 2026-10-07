---
title: "ADR-0001: Scaffold Choices"
category: architecture
purpose: Why findex was scaffolded as Tauri + React + pnpm with tray, updater, sqlite and titlebar.
status: active
updated: 2026-10-07
source: conversation
confidence: confirmed
verified_at: 2026-10-07
keywords: skill-workflow-init-repo, tauri, react, pnpm, sqlite, updater, public repo
---

# ADR-0001: Scaffold Choices

> Decided by the user on 2026-10-07 while scaffolding with `skill-workflow-init-repo`
> at v0.1.0, MIT, GitHub CI, bundle id `com.dtbao.findex`.

## Decision

| Choice | Reason given |
|--------|--------------|
| Tauri (Rust backend) | User requested a Tauri app |
| React 19 frontend | Recommended because the app will grow past a few screens |
| pnpm | Matches sibling apps (plan-dtbao, expense-tracker, ai-automatic-workflow) |
| tray, updater, titlebar | Picked by the user from the feature list |
| sqlite | Added after it was pointed out that folder tags and the content index must persist |
| Public GitHub repo | User asked for a public repository under `dtbao-embedded-dev` |

## Consequences

- The bundle id cannot change without orphaning installed copies.
- The updater needs a signing key and two repository secrets before the first release.
- The full-text index approach is still open (SQLite FTS5 vs a dedicated engine).
