//! Builds an index by walking folders, for drives without an NTFS file table, or when
//! the app may not read one. Sibling folders are read in parallel. Junctions and
//! symbolic links are listed but never entered, so a link back up the tree cannot loop.

use std::os::windows::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::error::{AppError, AppResult};
use crate::model::index::store::VolumeIndex;

/// The made-up reference number of the walked root; entries get 1, 2, 3, ...
pub const WALK_ROOT: u64 = 0;

/// How often, in entries, the walk reports progress.
const PROGRESS_EVERY: usize = 65_536;

struct Entry {
    id: u64,
    parent: u64,
    name: String,
    is_dir: bool,
}

/// What a walk reports to and asks of its caller, shared by every folder task.
pub struct Hooks<'a> {
    pub progress: &'a (dyn Fn(usize) + Sync),
    /// Checked before each folder; false ends the walk with `superseded`.
    pub keep_going: &'a (dyn Fn() -> bool + Sync),
}

/// Walks everything under `root`; paths in the index start with `display_root`.
pub fn build(root: &Path, display_root: &str, hooks: &Hooks<'_>) -> AppResult<VolumeIndex> {
    // A missing or unreadable root is an error; unreadable folders below it are not.
    std::fs::read_dir(root)?;
    // Its own threads: a walk keeps every worker busy for its whole length, and on
    // rayon's shared pool every search would queue behind it.
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .thread_name(|i| format!("findex-walk-{i}"))
        .build()
        .map_err(|err| AppError::Invalid(format!("could not start the folder scan: {err}")))?;
    let next = AtomicU64::new(WALK_ROOT + 1);
    let seen = AtomicUsize::new(0);
    let entries = pool.install(|| walk_dir(root.to_path_buf(), WALK_ROOT, &next, &seen, hooks));
    if !(hooks.keep_going)() {
        return Err(super::superseded());
    }
    let mut index = VolumeIndex::new(display_root, WALK_ROOT);
    index.reserve(entries.len(), entries.iter().map(|e| e.name.len()).sum());
    for entry in entries {
        index.upsert(entry.id, entry.parent, &entry.name, entry.is_dir)?;
    }
    Ok(index)
}

fn walk_dir(
    path: PathBuf,
    id: u64,
    next: &AtomicU64,
    seen: &AtomicUsize,
    hooks: &Hooks<'_>,
) -> Vec<Entry> {
    if !(hooks.keep_going)() {
        return Vec::new();
    }
    let Ok(listing) = std::fs::read_dir(&path) else {
        // Access denied and folders deleted mid-walk are skipped, not fatal.
        return Vec::new();
    };
    let mut entries = Vec::new();
    let mut folders = Vec::new();
    for item in listing.flatten() {
        let Ok(kind) = item.file_type() else {
            continue;
        };
        let child = next.fetch_add(1, Ordering::Relaxed);
        let is_link = kind.is_symlink();
        let is_dir = kind.is_dir() || kind.is_symlink_dir();
        if is_dir && !is_link {
            folders.push((item.path(), child));
        }
        entries.push(Entry {
            id: child,
            parent: id,
            name: item.file_name().to_string_lossy().into_owned(),
            is_dir,
        });
    }
    let before = seen.fetch_add(entries.len(), Ordering::Relaxed);
    if (before + entries.len()) / PROGRESS_EVERY > before / PROGRESS_EVERY {
        (hooks.progress)(before + entries.len());
    }
    let nested: Vec<Vec<Entry>> = folders
        .into_par_iter()
        .map(|(folder, child)| walk_dir(folder, child, next, seen, hooks))
        .collect();
    entries.extend(nested.into_iter().flatten());
    entries
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use super::*;
    use crate::model::search;

    const QUIET: Hooks<'static> = Hooks {
        progress: &|_| {},
        keep_going: &|| true,
    };

    #[test]
    fn a_superseded_walk_stops_with_an_error() {
        let temp = tree("stopped");
        let hooks = Hooks {
            progress: &|_| {},
            keep_going: &|| false,
        };
        assert!(matches!(
            build(&temp.0, "T:\\", &hooks),
            Err(AppError::Invalid(_))
        ));
    }

    /// A temp folder removed when the test ends, pass or fail.
    struct TempTree(PathBuf);

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn tree(name: &str) -> TempTree {
        let root = std::env::temp_dir().join(format!("findex-walk-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs").join("specs")).unwrap();
        fs::write(root.join("docs").join("specs").join("uart.md"), "x").unwrap();
        fs::write(root.join("docs").join("readme.txt"), "x").unwrap();
        fs::write(root.join("top.c"), "x").unwrap();
        TempTree(root)
    }

    #[test]
    fn indexes_every_entry_with_full_paths() {
        let temp = tree("paths");
        let index = build(&temp.0, "T:\\", &QUIET).unwrap();
        assert_eq!(index.len(), 5);
        let hits = search::run(&[index], &search::parse("uart"), 10, &|| false).unwrap();
        assert_eq!(hits[0].path, "T:\\docs\\specs\\uart.md");
    }

    #[test]
    fn lists_a_junction_without_entering_it() {
        let temp = tree("junction");
        // A junction back to the root: following it would loop forever.
        let link = temp.0.join("docs").join("loop");
        let made = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&temp.0)
            .output()
            .unwrap();
        assert!(made.status.success(), "mklink failed: {made:?}");
        let index = build(&temp.0, "T:\\", &QUIET).unwrap();
        assert_eq!(index.len(), 6);
        let hits = search::run(&[index], &search::parse("loop"), 10, &|| false).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].is_dir);
    }

    #[test]
    fn a_missing_root_is_an_error() {
        let missing = std::env::temp_dir().join("findex-walk-missing-root-does-not-exist");
        assert!(build(&missing, "T:\\", &QUIET).is_err());
    }
}
