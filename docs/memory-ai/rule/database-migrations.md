---
title: Database Migrations
category: rule
purpose: How to change the SQLite schema without breaking installed copies.
status: active
updated: 2026-10-07
source: src-tauri/src/model/db.rs
confidence: confirmed
verified_at: 2026-10-07
keywords: MIGRATIONS, migrate, user_version, rusqlite, Db
---

# Database Migrations

> Change the schema only by appending a new entry to `MIGRATIONS`; never edit or reorder
> a shipped entry.

## Rule

- Entry N of `MIGRATIONS` produces schema version N, recorded in `PRAGMA user_version`.
- `migrate` skips entries at or below the stored version and runs each remaining one in
  its own transaction, so a shipped entry has already run on users' machines.
- Migration 2 (2026-10-07) adds `folder_tag`; its test upgrades a version-1 database
  and checks that the settings survive. The content index will arrive the same way.

## See also

- [App shell](../architecture/app-shell.md)
