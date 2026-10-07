//! What a result is, how big it is and when it last changed, read from the disk when
//! results are shown. The index holds names only: reading these for every entry
//! would slow indexing, while a page of results takes a few milliseconds.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::UNIX_EPOCH;

use rayon::prelude::*;
use serde::Serialize;

use crate::model::win32;

/// One result's details, as the result row shows them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub path: String,
    /// Explorer's name for the type, such as "PDF Document" or "File folder".
    pub kind: String,
    /// Bytes, for files only.
    pub size: Option<u64>,
    /// Last change, in milliseconds since the Unix epoch.
    pub modified: Option<i64>,
}

/// The details of every path that still exists, in the order given; a path deleted
/// since it was indexed is left out rather than reported as an error.
pub fn describe(paths: &[String]) -> Vec<Details> {
    // Reading metadata waits on the disk, so the reads overlap.
    let found: Vec<(&String, std::fs::Metadata)> = paths
        .par_iter()
        .filter_map(|path| std::fs::metadata(path).ok().map(|meta| (path, meta)))
        .collect();
    found
        .into_iter()
        .map(|(path, meta)| {
            let ext = (!meta.is_dir()).then(|| {
                Path::new(path)
                    .extension()
                    .map_or_else(String::new, |ext| ext.to_string_lossy().to_lowercase())
            });
            Details {
                path: path.clone(),
                kind: kind_of(ext.as_deref()),
                size: ext.is_some().then_some(meta.len()),
                modified: meta
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .and_then(|since| i64::try_from(since.as_millis()).ok()),
            }
        })
        .collect()
}

/// The type name for an extension (`None` for a folder), asked of the shell once
/// per extension and then remembered: a page of results repeats a few types.
fn kind_of(ext: Option<&str>) -> String {
    static KINDS: OnceLock<Mutex<HashMap<Option<String>, String>>> = OnceLock::new();
    let key = ext.map(str::to_owned);
    let kinds = KINDS.get_or_init(Mutex::default);
    if let Some(kind) = kinds
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
    {
        return kind.clone();
    }
    let kind = win32::file_type_name(ext).unwrap_or_else(|| match ext {
        Some(ext) => fallback_kind(ext),
        None => "File folder".to_owned(),
    });
    kinds
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, kind.clone());
    kind
}

/// Explorer's wording for a type the shell has no name for: "PDF File", or "File".
fn fallback_kind(ext: &str) -> String {
    if ext.is_empty() {
        "File".to_owned()
    } else {
        format!("{} File", ext.to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("findex-details-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reads_size_modified_and_type_of_a_file() {
        let dir = scratch("file");
        let file = dir.join("notes.txt");
        std::fs::write(&file, b"twelve bytes").unwrap();
        let when = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(when)
            .unwrap();

        let found = describe(&[file.display().to_string()]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, file.display().to_string());
        assert_eq!(found[0].size, Some(12));
        assert_eq!(found[0].modified, Some(1_700_000_000_000));
        assert!(!found[0].kind.is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_folder_has_a_type_and_date_but_no_size() {
        let dir = scratch("folder");
        let (a, b) = (dir.join("a"), dir.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let found = describe(&[a.display().to_string(), b.display().to_string()]);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].size, None);
        assert!(found[0].modified.is_some());
        assert!(!found[0].kind.is_empty());
        assert_eq!(found[0].kind, found[1].kind);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn leaves_out_a_path_that_no_longer_exists() {
        let dir = scratch("missing");
        let kept = dir.join("kept.md");
        std::fs::write(&kept, b"").unwrap();
        let gone = dir.join("gone.md").display().to_string();
        let found = describe(&[gone, kept.display().to_string()]);
        assert_eq!(
            found.iter().map(|d| d.path.clone()).collect::<Vec<_>>(),
            [kept.display().to_string()]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_unknown_type_is_named_after_its_extension() {
        assert_eq!(fallback_kind("xyz"), "XYZ File");
        assert_eq!(fallback_kind(""), "File");
    }
}
