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
- Folder tags and the content index will arrive as new migrations, each with a unit
  test on an in-memory database like the existing three in `db.rs`.

## See also

- [App shell](../architecture/app-shell.md)
