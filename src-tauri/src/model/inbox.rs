//! Tag requests from `findex-tag`, the helper Explorer's "Tag with Findex" menu
//! runs. The helper cannot open the database (it runs unelevated, Findex does not),
//! so it drops one `.req` file per click into the inbox folder: the tag on the first
//! line, the folder on the second. Findex applies them here, at start and then
//! every second, and deletes each file once read, applied or not.

use std::ffi::OsStr;
use std::path::Path;

use crate::error::AppResult;
use crate::model::db::Db;
use crate::model::tags;

/// A request is two short lines; anything bigger was not written by the helper.
const MAX_REQUEST: u64 = 4096;

/// Applies every request waiting in `dir` and returns how many tagged a folder. A
/// request that cannot be applied (a bad tag, a folder deleted since) is dropped:
/// retrying it would fail the same way. `.tmp` files are still being written and
/// are left for the next pass; a missing folder means nothing was ever asked.
pub fn drain(db: &Db, dir: &Path) -> AppResult<usize> {
    // The inbox is in the user's own profile, writable without elevation: never
    // follow it through a link to somewhere else.
    match std::fs::symlink_metadata(dir) {
        Ok(meta) if meta.is_dir() => {}
        _ => return Ok(0),
    }
    let mut applied = 0;
    for entry in std::fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension() != Some(OsStr::new("req")) {
            continue;
        }
        // Only plain files: a link would make Findex read, as administrator,
        // whatever it points at.
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let request = (meta.len() <= MAX_REQUEST)
            .then(|| std::fs::read_to_string(&path).ok())
            .flatten();
        if let Some((tag, folder)) = request.as_deref().and_then(parse) {
            match tags::add(db, folder, tag) {
                Ok(_) => applied += 1,
                Err(err) => eprintln!("could not apply the tag request {}: {err}", path.display()),
            }
        }
        if let Err(err) = std::fs::remove_file(&path) {
            eprintln!("could not remove the tag request {}: {err}", path.display());
        }
    }
    Ok(applied)
}

/// The tag and the folder of a request, or `None` when either line is missing.
fn parse(text: &str) -> Option<(&str, &str)> {
    let mut lines = text.lines().map(str::trim);
    let tag = lines.next().filter(|line| !line.is_empty())?;
    let folder = lines.next().filter(|line| !line.is_empty())?;
    Some((tag, folder))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::db::open_in_memory;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("findex-inbox-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("inbox")).unwrap();
        dir
    }

    #[test]
    fn a_request_tags_its_folder_and_is_removed() {
        let db = open_in_memory().unwrap();
        let root = scratch("apply");
        let folder = root.join("work");
        std::fs::create_dir_all(&folder).unwrap();
        let inbox = root.join("inbox");
        std::fs::write(
            inbox.join("1-1.req"),
            format!("working\r\n{}\r\n", folder.display()),
        )
        .unwrap();

        assert_eq!(drain(&db, &inbox).unwrap(), 1);
        let tagged = tags::for_paths(&db, &[folder.display().to_string()]).unwrap();
        assert_eq!(tagged[&folder.display().to_string()], ["working"]);
        assert_eq!(std::fs::read_dir(&inbox).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_request_that_cannot_be_applied_is_dropped() {
        let db = open_in_memory().unwrap();
        let root = scratch("bad");
        let inbox = root.join("inbox");
        std::fs::write(inbox.join("1.req"), "two words\nC:\\\n").unwrap();
        std::fs::write(inbox.join("2.req"), "doc\nZ:\\no\\such\\folder\n").unwrap();
        std::fs::write(inbox.join("3.req"), "doc\n").unwrap();
        std::fs::write(inbox.join("4.req"), "x".repeat(5000)).unwrap();

        assert_eq!(drain(&db, &inbox).unwrap(), 0);
        assert_eq!(std::fs::read_dir(&inbox).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_request_still_being_written_is_left_for_later() {
        let db = open_in_memory().unwrap();
        let root = scratch("tmp");
        let inbox = root.join("inbox");
        std::fs::write(inbox.join("1.tmp"), "doc\n").unwrap();

        assert_eq!(drain(&db, &inbox).unwrap(), 0);
        assert!(inbox.join("1.tmp").exists());
        // No inbox yet: nothing was ever asked for.
        assert_eq!(drain(&db, &root.join("missing")).unwrap(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
