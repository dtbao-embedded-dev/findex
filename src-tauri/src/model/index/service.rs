//! Keeps the chosen drives indexed and answers searches over them.
//!
//! Each drive is built on its own thread: from the NTFS file table when the app may
//! read it, otherwise by walking folders. A drive's old index stays searchable until
//! its new one replaces it. Starting again (after the drive choice changes) makes
//! every unfinished build of the previous start stale: its result is dropped.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};

use serde::Serialize;

use crate::error::AppResult;
use crate::model::db::Db;
use crate::model::drives::{self, Method};
use crate::model::index::store::VolumeIndex;
use crate::model::index::{mft, walk};
use crate::model::search::{self, Hit};
use crate::model::win32;

/// How the service reports progress; the app turns it into an `index-status` event.
pub type Emit = Arc<dyn Fn(&Status) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DriveState {
    Indexing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveStatus {
    pub letter: String,
    pub state: DriveState,
    pub entries: usize,
    /// How the drive was actually read, which can differ from what it supports.
    pub method: Method,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub drives: Vec<DriveStatus>,
}

#[derive(Default)]
pub struct IndexService {
    volumes: RwLock<Vec<VolumeIndex>>,
    status: Mutex<Status>,
    /// Bumped by every search; a running search stops when it no longer matches.
    query: AtomicU64,
    /// Bumped by every start; a build finishing under an older value is discarded.
    run: AtomicU64,
}

impl IndexService {
    /// Indexes the drives saved in setup; does nothing before setup has been done.
    pub fn start_saved(self: &Arc<Self>, db: &Db, emit: Emit) -> AppResult<()> {
        if let Some(letters) = drives::load_selection(db)? {
            self.start(&letters, emit);
        }
        Ok(())
    }

    /// (Re)indexes exactly `letters`, dropping drives no longer chosen.
    pub fn start(self: &Arc<Self>, letters: &[String], emit: Emit) {
        let run = self.run.fetch_add(1, Ordering::SeqCst) + 1;
        let chosen: Vec<char> = letters.iter().filter_map(|l| l.chars().next()).collect();
        self.write_volumes()
            .retain(|v| chosen.contains(&root_letter(v)));
        let filesystems = win32::list_volumes();
        let plan: Vec<(char, Method)> = chosen
            .iter()
            .map(|&letter| {
                let filesystem = filesystems
                    .iter()
                    .find(|v| v.letter == letter)
                    .map_or("", |v| v.filesystem.as_str());
                (letter, drives::method_for(filesystem))
            })
            .collect();
        let snapshot = {
            let mut status = self.lock_status();
            status.drives = plan
                .iter()
                .map(|&(letter, method)| DriveStatus {
                    letter: letter.to_string(),
                    state: DriveState::Indexing,
                    entries: 0,
                    method,
                    note: None,
                })
                .collect();
            status.clone()
        };
        emit(&snapshot);
        for (letter, method) in plan {
            let service = Arc::clone(self);
            let emit = Arc::clone(&emit);
            std::thread::spawn(move || service.index_drive(run, letter, method, &emit));
        }
    }

    pub fn status(&self) -> Status {
        self.lock_status().clone()
    }

    /// Searches every ready drive. Returns `None` when a newer search replaced this
    /// one before it finished, so the caller can drop it.
    pub fn search(&self, text: &str, limit: usize) -> Option<Vec<Hit>> {
        let mine = self.query.fetch_add(1, Ordering::SeqCst) + 1;
        let query = search::parse(text);
        let volumes = self.volumes.read().unwrap_or_else(PoisonError::into_inner);
        search::run(&volumes, &query, limit, &|| {
            self.query.load(Ordering::Relaxed) != mine
        })
    }

    fn index_drive(&self, run: u64, letter: char, method: Method, emit: &Emit) {
        let progress = |entries: usize| self.update(run, letter, emit, |d| d.entries = entries);
        let built = match method {
            Method::Mft => match mft::build(letter, &mut |n| progress(n)) {
                Ok(built) => Ok((built.index, Method::Mft, None)),
                Err(err) => {
                    let note = format!("Folder scan: the file table could not be read ({err})");
                    walk_drive(letter, &progress).map(|index| (index, Method::Walk, Some(note)))
                }
            },
            Method::Walk => walk_drive(letter, &progress).map(|index| (index, Method::Walk, None)),
        };
        match built {
            Ok((index, used, note)) => {
                let entries = index.len();
                if self.install(run, index) {
                    self.update(run, letter, emit, |d| {
                        d.state = DriveState::Ready;
                        d.entries = entries;
                        d.method = used;
                        d.note = note;
                    });
                }
            }
            Err(err) => self.update(run, letter, emit, |d| {
                d.state = DriveState::Failed;
                d.note = Some(err.to_string());
            }),
        }
    }

    /// Replaces the drive's index with `index`, unless a newer start superseded `run`.
    fn install(&self, run: u64, index: VolumeIndex) -> bool {
        let mut volumes = self.write_volumes();
        if self.run.load(Ordering::SeqCst) != run {
            return false;
        }
        let letter = root_letter(&index);
        volumes.retain(|v| root_letter(v) != letter);
        volumes.push(index);
        volumes.sort_by_key(root_letter);
        true
    }

    fn update(&self, run: u64, letter: char, emit: &Emit, change: impl FnOnce(&mut DriveStatus)) {
        let snapshot = {
            let mut status = self.lock_status();
            if self.run.load(Ordering::SeqCst) != run {
                return;
            }
            let key = letter.to_string();
            match status.drives.iter_mut().find(|d| d.letter == key) {
                Some(drive) => change(drive),
                None => return,
            }
            status.clone()
        };
        emit(&snapshot);
    }

    fn write_volumes(&self) -> std::sync::RwLockWriteGuard<'_, Vec<VolumeIndex>> {
        self.volumes.write().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_status(&self) -> MutexGuard<'_, Status> {
        self.status.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn walk_drive(letter: char, progress: &(dyn Fn(usize) + Sync)) -> AppResult<VolumeIndex> {
    let root = format!("{letter}:\\");
    walk::build(&PathBuf::from(&root), &root, progress)
}

fn root_letter(index: &VolumeIndex) -> char {
    index.root().chars().next().unwrap_or('?')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume(letter: char, names: &[&str]) -> VolumeIndex {
        let mut index = VolumeIndex::new(format!("{letter}:\\"), 0);
        for (i, name) in names.iter().enumerate() {
            index.upsert(i as u64 + 1, 0, name, false).unwrap();
        }
        index
    }

    fn quiet() -> Emit {
        Arc::new(|_| {})
    }

    #[test]
    fn searches_installed_drives_in_letter_order() {
        let service = IndexService::default();
        assert!(service.install(0, volume('D', &["plan.md"])));
        assert!(service.install(0, volume('C', &["plan.md"])));
        let paths: Vec<_> = service
            .search("plan", 10)
            .unwrap()
            .into_iter()
            .map(|hit| hit.path)
            .collect();
        assert_eq!(paths, ["C:\\plan.md", "D:\\plan.md"]);
    }

    #[test]
    fn a_rebuilt_drive_replaces_its_old_index() {
        let service = IndexService::default();
        service.install(0, volume('C', &["old.txt"]));
        service.install(0, volume('C', &["new.txt"]));
        assert!(service.search("old", 10).unwrap().is_empty());
        assert_eq!(service.search("new", 10).unwrap().len(), 1);
    }

    #[test]
    fn a_stale_build_is_discarded() {
        let service = Arc::new(IndexService::default());
        service.start(&[], quiet());
        assert!(!service.install(0, volume('C', &["stale.txt"])));
        assert!(service.search("stale", 10).unwrap().is_empty());
    }

    #[test]
    fn starting_with_fewer_drives_drops_the_others() {
        let service = Arc::new(IndexService::default());
        service.install(0, volume('C', &["a.txt"]));
        service.start(&[], quiet());
        assert!(service.search("a", 10).unwrap().is_empty());
        assert_eq!(service.status(), Status::default());
    }

    #[test]
    fn progress_of_a_superseded_run_is_not_reported() {
        let service = Arc::new(IndexService::default());
        let seen = Arc::new(Mutex::new(Vec::<Status>::new()));
        let record = Arc::clone(&seen);
        let emit: Emit = Arc::new(move |status| record.lock().unwrap().push(status.clone()));
        service.start(&[], Arc::clone(&emit));
        service.update(0, 'C', &emit, |d| d.entries = 99);
        assert_eq!(seen.lock().unwrap().len(), 1);
    }
}
