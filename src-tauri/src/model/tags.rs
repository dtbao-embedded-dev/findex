//! Tags on folders: short words a folder can be found by, stored in the `folder_tag`
//! table. A folder is keyed by its full path, compared without case, so a folder
//! that is renamed or moved loses its tags.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::params_from_iter;
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::model::db::Db;

/// The longest tag, in characters: a tag is a word, not a note.
const MAX_TAG: usize = 32;

/// A tag and how many folders carry it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
    pub tag: String,
    pub folders: u32,
}

/// Adds `tag` to the folder at `path`; returns the folder's tags, sorted. Adding a
/// tag it already has changes nothing.
pub fn add(db: &Db, path: &str, tag: &str) -> AppResult<Vec<String>> {
    let tag = normalize_tag(tag)?;
    let path = folder_key(path);
    if !Path::new(&path).is_dir() {
        return Err(AppError::Invalid(format!(
            "only a folder that exists can be tagged: {path}"
        )));
    }
    let conn = db.conn()?;
    conn.execute(
        "INSERT OR IGNORE INTO folder_tag (path, tag) VALUES (?1, ?2)",
        [&path, &tag],
    )?;
    tags_of(&conn, &path)
}

/// Removes `tag` from the folder at `path`; returns the tags left, sorted.
pub fn remove(db: &Db, path: &str, tag: &str) -> AppResult<Vec<String>> {
    let tag = normalize_tag(tag)?;
    let path = folder_key(path);
    let conn = db.conn()?;
    conn.execute(
        "DELETE FROM folder_tag WHERE path = ?1 AND tag = ?2",
        [&path, &tag],
    )?;
    tags_of(&conn, &path)
}

/// The tags of each given path that has any, keyed by the path as given.
pub fn for_paths(db: &Db, paths: &[String]) -> AppResult<HashMap<String, Vec<String>>> {
    if paths.is_empty() {
        return Ok(HashMap::new());
    }
    let keys: Vec<String> = paths.iter().map(|p| folder_key(p)).collect();
    let marks = vec!["?"; keys.len()].join(",");
    let conn = db.conn()?;
    let mut statement = conn.prepare(&format!(
        "SELECT path, tag FROM folder_tag WHERE path IN ({marks}) ORDER BY tag"
    ))?;
    let mut by_key: HashMap<String, Vec<String>> = HashMap::new();
    let rows = statement.query_map(params_from_iter(&keys), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (path, tag) = row?;
        by_key.entry(path.to_lowercase()).or_default().push(tag);
    }
    Ok(paths
        .iter()
        .zip(&keys)
        .filter_map(|(path, key)| {
            by_key
                .get(&key.to_lowercase())
                .map(|t| (path.clone(), t.clone()))
        })
        .collect())
}

/// Every tag in use with its folder count, most used first.
pub fn all(db: &Db) -> AppResult<Vec<TagCount>> {
    let conn = db.conn()?;
    let mut statement = conn
        .prepare("SELECT tag, COUNT(*) FROM folder_tag GROUP BY tag ORDER BY COUNT(*) DESC, tag")?;
    let rows = statement.query_map([], |row| {
        Ok(TagCount {
            tag: row.get(0)?,
            folders: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The folders that carry every one of `tags`.
pub fn folders_with(db: &Db, tags: &[String]) -> AppResult<Vec<String>> {
    let tags: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    let marks = vec!["?"; tags.len()].join(",");
    let conn = db.conn()?;
    let mut statement = conn.prepare(&format!(
        "SELECT path FROM folder_tag WHERE tag IN ({marks}) \
         GROUP BY path HAVING COUNT(DISTINCT tag) = {}",
        tags.len()
    ))?;
    let rows = statement.query_map(params_from_iter(&tags), |row| row.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// A tag as stored: trimmed, without a leading `#`, lower case; 1 to 32 letters,
/// digits, `-` or `_`.
pub fn normalize_tag(tag: &str) -> AppResult<String> {
    let tag = tag.trim().trim_start_matches('#').to_lowercase();
    let valid = !tag.is_empty()
        && tag.chars().count() <= MAX_TAG
        && tag
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(tag)
    } else {
        Err(AppError::Invalid(format!(
            "a tag is one word of up to {MAX_TAG} letters, digits, - or _"
        )))
    }
}

/// The path a folder is stored under: no trailing backslash, except on a drive root.
fn folder_key(path: &str) -> String {
    let trimmed = path.trim().trim_end_matches('\\');
    if trimmed.len() == 2 && trimmed.ends_with(':') {
        format!("{trimmed}\\")
    } else {
        trimmed.to_owned()
    }
}

fn tags_of(conn: &rusqlite::Connection, path: &str) -> AppResult<Vec<String>> {
    let mut statement = conn.prepare("SELECT tag FROM folder_tag WHERE path = ?1 ORDER BY tag")?;
    let rows = statement.query_map([path], |row| row.get(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::db;

    fn folder(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("findex-tags-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.display().to_string()
    }

    #[test]
    fn a_tag_is_one_short_lowercase_word() {
        assert_eq!(normalize_tag("  #Work ").unwrap(), "work");
        assert_eq!(normalize_tag("esp-idf_v5").unwrap(), "esp-idf_v5");
        assert_eq!(normalize_tag("Ảnh").unwrap(), "ảnh");
        for bad in ["", "#", "two words", "a;b", &"x".repeat(33)] {
            assert!(
                matches!(normalize_tag(bad), Err(AppError::Invalid(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn tags_are_added_once_listed_sorted_and_removed() {
        let db = db::open_in_memory().unwrap();
        let dir = folder("add");
        assert_eq!(add(&db, &dir, "work").unwrap(), ["work"]);
        assert_eq!(add(&db, &dir, "#Esp32").unwrap(), ["esp32", "work"]);
        assert_eq!(add(&db, &dir, "WORK").unwrap(), ["esp32", "work"]);
        assert_eq!(remove(&db, &dir, "work").unwrap(), ["esp32"]);
        assert_eq!(remove(&db, &dir, "work").unwrap(), ["esp32"]);
    }

    #[test]
    fn only_a_folder_that_exists_can_be_tagged() {
        let db = db::open_in_memory().unwrap();
        let file = std::env::current_exe().unwrap().display().to_string();
        assert!(matches!(add(&db, &file, "x"), Err(AppError::Invalid(_))));
        assert!(matches!(
            add(&db, r"Z:\no\such", "x"),
            Err(AppError::Invalid(_))
        ));
    }

    #[test]
    fn looks_tags_up_by_path_ignoring_case_and_a_trailing_backslash() {
        let db = db::open_in_memory().unwrap();
        let dir = folder("lookup");
        add(&db, &format!("{dir}\\"), "work").unwrap();
        let upper = dir.to_uppercase();
        let found = for_paths(&db, &[upper.clone(), "C:\\elsewhere".into()]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[&upper], ["work"]);
    }

    #[test]
    fn counts_tags_and_finds_folders_carrying_all_of_them() {
        let db = db::open_in_memory().unwrap();
        let (a, b) = (folder("all-a"), folder("all-b"));
        add(&db, &a, "work").unwrap();
        add(&db, &a, "fw").unwrap();
        add(&db, &b, "work").unwrap();
        assert_eq!(
            all(&db).unwrap(),
            [
                TagCount {
                    tag: "work".into(),
                    folders: 2
                },
                TagCount {
                    tag: "fw".into(),
                    folders: 1
                },
            ]
        );
        let mut both = folders_with(&db, &["work".into()]).unwrap();
        both.sort();
        let mut expected = vec![a.clone(), b];
        expected.sort();
        assert_eq!(both, expected);
        assert_eq!(
            folders_with(&db, &["work".into(), "FW".into()]).unwrap(),
            [a]
        );
        assert!(folders_with(&db, &["none".into()]).unwrap().is_empty());
    }
}
