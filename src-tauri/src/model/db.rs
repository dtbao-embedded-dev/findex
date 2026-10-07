//! The app's SQLite database: one file in the app data directory, its schema moved
//! forward by numbered migrations recorded in `PRAGMA user_version`.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension};

use crate::error::{AppError, AppResult};

/// Applied in order; the Nth entry produces schema version N. Append, never edit:
/// a shipped migration has already run on someone's machine.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    // Folder tags; a path matches without case, as Windows paths do.
    "CREATE TABLE folder_tag (\
         path TEXT NOT NULL COLLATE NOCASE, \
         tag TEXT NOT NULL, \
         PRIMARY KEY (path, tag)); \
     CREATE INDEX folder_tag_by_tag ON folder_tag (tag);",
    // The tag catalog: every tag that can be picked, carried by a folder or not. The
    // colour tags ship with the app, cannot be deleted and sort first; `color` names
    // the colour, which the frontend draws for light or dark. Tags already on folders
    // become custom tags.
    "CREATE TABLE tag (\
         name TEXT PRIMARY KEY, \
         color TEXT, \
         preset INTEGER NOT NULL DEFAULT 0, \
         ord INTEGER NOT NULL DEFAULT 1000); \
     INSERT INTO tag (name, color, preset, ord) VALUES \
         ('red', 'red', 1, 1), ('orange', 'orange', 1, 2), ('yellow', 'yellow', 1, 3), \
         ('green', 'green', 1, 4), ('blue', 'blue', 1, 5), ('purple', 'purple', 1, 6), \
         ('gray', 'gray', 1, 7); \
     INSERT OR IGNORE INTO tag (name) SELECT DISTINCT tag FROM folder_tag;",
];

/// The connection every command shares through `tauri::State`.
pub struct Db(Mutex<Connection>);

/// Opens the database at `path`, creating it and its folder if needed, and migrates it.
pub fn open(path: &Path) -> AppResult<Db> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut conn = Connection::open(path)?;
    migrate(&mut conn)?;
    Ok(Db(Mutex::new(conn)))
}

/// A migrated database that lives only as long as the value, for tests of other modules.
#[cfg(test)]
pub fn open_in_memory() -> AppResult<Db> {
    let mut conn = Connection::open_in_memory()?;
    migrate(&mut conn)?;
    Ok(Db(Mutex::new(conn)))
}

/// Brings the schema to the latest version, each migration in its own transaction.
pub fn migrate(conn: &mut Connection) -> AppResult<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    for (version, sql) in (1_i64..).zip(MIGRATIONS) {
        if version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
    }
    Ok(())
}

impl Db {
    pub(crate) fn conn(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.0
            .lock()
            .map_err(|_| AppError::Invalid("the database is unavailable after a crash".into()))
    }

    pub fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        let value = self
            .conn()?
            .query_row("SELECT value FROM setting WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(value)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn()?.execute(
            "INSERT INTO setting (key, value) VALUES (?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory() -> Db {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        Db(Mutex::new(conn))
    }

    fn version(db: &Db) -> i64 {
        db.conn()
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn migrates_to_the_latest_version() {
        assert_eq!(version(&memory()), MIGRATIONS.len() as i64);
    }

    #[test]
    fn migrating_again_changes_nothing() {
        let db = memory();
        migrate(&mut db.conn().unwrap()).unwrap();
        assert_eq!(version(&db), MIGRATIONS.len() as i64);
    }

    #[test]
    fn a_version_one_database_keeps_its_settings_and_gains_folder_tags() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute("INSERT INTO setting VALUES ('scan.drives', '[\"C\"]')", [])
            .unwrap();
        migrate(&mut conn).unwrap();
        let db = Db(Mutex::new(conn));
        assert_eq!(
            db.get_setting("scan.drives").unwrap().as_deref(),
            Some("[\"C\"]")
        );
        db.conn()
            .unwrap()
            .execute(
                "INSERT INTO folder_tag (path, tag) VALUES ('C:\\a', 'x')",
                [],
            )
            .unwrap();
    }

    #[test]
    fn a_version_two_database_keeps_its_tags_as_custom_ones() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.execute_batch(MIGRATIONS[1]).unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
        conn.execute_batch(
            "INSERT INTO folder_tag VALUES ('C:\\a', 'work'), ('C:\\b', 'work'), ('C:\\a', 'red');",
        )
        .unwrap();
        migrate(&mut conn).unwrap();
        let rows: Vec<(String, Option<String>, bool)> = conn
            .prepare("SELECT name, color, preset FROM tag ORDER BY ord, name")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let names: Vec<&str> = rows.iter().map(|(name, _, _)| name.as_str()).collect();
        assert_eq!(
            names,
            ["red", "orange", "yellow", "green", "blue", "purple", "gray", "work"]
        );
        // A tag that was already a colour name stays the preset; the others are custom.
        assert_eq!(rows[0], ("red".into(), Some("red".into()), true));
        assert_eq!(rows[7], ("work".into(), None, false));
    }

    #[test]
    fn a_setting_round_trips_and_is_overwritten() {
        let db = memory();
        assert_eq!(db.get_setting("theme").unwrap(), None);
        db.set_setting("theme", "dark").unwrap();
        db.set_setting("theme", "light").unwrap();
        assert_eq!(db.get_setting("theme").unwrap().as_deref(), Some("light"));
    }
}
