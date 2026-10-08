//! How much a folder holds: the sum of the file sizes under it, read from the disk
//! for the folder results on screen. The index holds names only, so this walks the
//! folder each time; a drive's root can take minutes.
//!
//! - **Links:** junctions and symbolic links are never followed, so a link back up
//!   the tree cannot loop and a link to another drive is not counted.
//! - **Unreadable folders** below the one asked for count as empty.
//! - **Cancelling:** each request replaces the one before: the rows on screen
//!   changed, so the walks for rows scrolled away stop.
//! - **Threads:** a pool of its own, so a long walk never slows name searches.
//! - **Cache:** a size is kept for five minutes, so scrolling back is instant.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use rayon::prelude::*;
use serde::Serialize;

use crate::error::{AppError, AppResult};

/// How long a measured size is shown without walking the folder again.
const KEEP_FOR: Duration = Duration::from_secs(5 * 60);

/// Walks at once; fewer than the cores, so the disk and the search keep up.
const THREADS: usize = 4;

/// One folder's size, as the frontend receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FolderSize {
    pub path: String,
    pub bytes: u64,
}

/// Measures folders on its own threads and remembers the answers.
pub struct Sizer {
    pool: rayon::ThreadPool,
    /// Bumped by every request; a walk stops once it is no longer the latest.
    latest: AtomicU64,
    cache: Mutex<HashMap<String, (u64, Instant)>>,
}

impl Sizer {
    pub fn new() -> AppResult<Self> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(THREADS)
            .build()
            .map_err(|err| AppError::Invalid(format!("folder sizes cannot be read: {err}")))?;
        Ok(Self {
            pool,
            latest: AtomicU64::new(0),
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Sends each folder's size to `send` as soon as it is known: remembered ones
    /// first, then each walk as it ends. Stops the walks of any earlier request,
    /// and is stopped in turn by the next one. A folder that cannot be read sends
    /// nothing.
    pub fn sizes(&self, paths: &[String], send: &(dyn Fn(FolderSize) + Sync)) {
        let mine = self.latest.fetch_add(1, Ordering::SeqCst) + 1;
        let current = || self.latest.load(Ordering::SeqCst) == mine;
        let mut todo = Vec::new();
        for path in paths {
            match self.remembered(path) {
                Some(bytes) => send(FolderSize {
                    path: path.clone(),
                    bytes,
                }),
                None => todo.push(path),
            }
        }
        self.pool.install(|| {
            todo.par_iter().for_each(|path| {
                if let Some(bytes) = size_of(Path::new(path.as_str()), &current) {
                    self.remember(path, bytes);
                    send(FolderSize {
                        path: (*path).clone(),
                        bytes,
                    });
                }
            });
        });
    }

    fn remembered(&self, path: &str) -> Option<u64> {
        let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache
            .get(&path.to_lowercase())
            .filter(|(_, at)| at.elapsed() < KEEP_FOR)
            .map(|(bytes, _)| *bytes)
    }

    fn remember(&self, path: &str, bytes: u64) {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.retain(|_, (_, at)| at.elapsed() < KEEP_FOR);
        cache.insert(path.to_lowercase(), (bytes, Instant::now()));
    }
}

/// The bytes of every file under `dir`, links not followed; `None` when `dir`
/// cannot be read or `keep_going` turned false before the walk ended.
pub fn size_of(dir: &Path, keep_going: &(dyn Fn() -> bool + Sync)) -> Option<u64> {
    let listing = std::fs::read_dir(dir).ok()?;
    walk(listing, keep_going)
}

fn walk(listing: std::fs::ReadDir, keep_going: &(dyn Fn() -> bool + Sync)) -> Option<u64> {
    if !keep_going() {
        return None;
    }
    let mut bytes = 0u64;
    let mut folders = Vec::new();
    for item in listing.flatten() {
        let Ok(kind) = item.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            folders.push(item.path());
        } else if let Ok(meta) = item.metadata() {
            bytes += meta.len();
        }
    }
    let below = folders
        .par_iter()
        .map(|folder| match std::fs::read_dir(folder) {
            Ok(listing) => walk(listing, keep_going),
            // Access denied, or deleted mid-walk: counts as empty.
            Err(_) => Some(0),
        })
        .try_reduce(|| 0, |a, b| Some(a + b))?;
    Some(bytes + below)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn tree(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("findex-size-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a").join("b")).unwrap();
        std::fs::write(root.join("top.txt"), [0u8; 10]).unwrap();
        std::fs::write(root.join("a").join("mid.bin"), [0u8; 200]).unwrap();
        std::fs::write(root.join("a").join("b").join("deep.bin"), [0u8; 3000]).unwrap();
        root
    }

    #[test]
    fn adds_up_every_file_below() {
        let root = tree("sum");
        assert_eq!(size_of(&root, &|| true), Some(3210));
        assert_eq!(size_of(&root.join("a"), &|| true), Some(3200));
        assert_eq!(size_of(&root.join("missing"), &|| true), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn never_follows_a_junction() {
        let root = tree("junction");
        // A junction back to the root: following it would count the tree forever.
        let made = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(root.join("a").join("loop"))
            .arg(&root)
            .output()
            .unwrap();
        assert!(made.status.success(), "mklink failed: {made:?}");
        assert_eq!(size_of(&root, &|| true), Some(3210));
        std::fs::remove_dir(root.join("a").join("loop")).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_cancelled_walk_gives_no_size() {
        let root = tree("cancel");
        assert_eq!(size_of(&root, &|| false), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sends_each_size_and_remembers_it() {
        let root = tree("cache");
        let path = root.display().to_string();
        let sizer = Sizer::new().unwrap();
        let sent = Mutex::new(Vec::new());
        sizer.sizes(std::slice::from_ref(&path), &|size| {
            sent.lock().unwrap().push(size)
        });
        // Changed since: the remembered size is sent, without a walk.
        std::fs::write(root.join("new.bin"), [0u8; 5]).unwrap();
        sizer.sizes(&[path.to_uppercase()], &|size| {
            sent.lock().unwrap().push(size)
        });
        assert_eq!(
            *sent.lock().unwrap(),
            [
                FolderSize {
                    path: path.clone(),
                    bytes: 3210
                },
                FolderSize {
                    path: path.to_uppercase(),
                    bytes: 3210
                },
            ]
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
