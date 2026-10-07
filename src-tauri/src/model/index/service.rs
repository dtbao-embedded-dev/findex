//! Keeps the chosen drives indexed and answers searches over them.
//!
//! Each drive is built on its own thread: from the NTFS file table when the app may
//! read it, otherwise by walking folders. A drive's old index stays searchable until
//! its new one replaces it. Starting again (after the drive choice changes) makes
//! every unfinished build of the previous start stale: its result is dropped.
//!
//! A finished build is saved as a snapshot; the next start loads it first, so the
//! drive is searchable at once. An NTFS drive whose journal still reaches back to
//! the snapshot is brought up to date from the journal instead of rebuilt, and is
//! then kept current from it for as long as the app runs.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::model::db::Db;
use crate::model::drives::{self, Method};
use crate::model::index::store::VolumeIndex;
use crate::model::index::{mft, snapshot, walk, watch};
use crate::model::search::{self, Hit};
use crate::model::win32::{self, Journal};

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
    /// Entries read so far by the running build; the full count once ready.
    pub entries: usize,
    /// Entries that can be searched right now: a loaded snapshot or the last build.
    pub searchable: usize,
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
    /// Where snapshots are kept; `None` keeps everything in memory only.
    snapshots: Option<PathBuf>,
    /// The journal position each followed drive's installed index is current to.
    positions: Mutex<HashMap<char, Journal>>,
    /// Drives changed by live updates since their snapshot was last saved.
    dirty: Mutex<HashSet<char>>,
    /// Held while a snapshot is written.
    saving: Mutex<()>,
}

/// Live updates send status at most this often, so a busy disk does not flood the UI.
const STATUS_EVERY: Duration = Duration::from_secs(1);

/// How often a followed drive's snapshot is saved while it keeps changing.
const SAVE_EVERY: Duration = Duration::from_secs(600);

impl IndexService {
    /// A service that saves and reloads snapshots in `dir`.
    pub fn with_snapshots(dir: PathBuf) -> Self {
        Self {
            snapshots: Some(dir),
            ..Self::default()
        }
    }

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
        let kept: Vec<(char, usize)> = {
            let mut volumes = self.write_volumes();
            volumes.retain(|v| chosen.contains(&root_letter(v)));
            volumes.iter().map(|v| (root_letter(v), v.len())).collect()
        };
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
                    searchable: kept
                        .iter()
                        .find(|(l, _)| *l == letter)
                        .map_or(0, |(_, n)| *n),
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

    /// Saves every drive changed by live updates since its last save; called on quit,
    /// so the next launch resumes from the journal instead of rebuilding.
    pub fn save_changed(&self) {
        let letters: Vec<char> = self.lock_dirty().drain().collect();
        for letter in letters {
            self.save_installed(letter);
        }
    }

    /// One drive's whole life: show its snapshot, bring it up to date (resuming from
    /// the journal when the snapshot allows, else rebuilding), then, for NTFS, follow
    /// the journal until a newer start takes over. A journal gap rebuilds and follows
    /// again.
    fn index_drive(&self, run: u64, letter: char, method: Method, emit: &Emit) {
        let saved = self.load_snapshot(run, letter, emit);
        let mut resume =
            saved.filter(|journal| method == Method::Mft && watch::continues(letter, journal));
        loop {
            let journal = match resume.take() {
                Some(journal) => {
                    self.update(run, letter, emit, |d| {
                        d.state = DriveState::Ready;
                        d.entries = d.searchable;
                        d.method = Method::Mft;
                    });
                    Some(journal)
                }
                None => match self.build(run, letter, method, emit) {
                    Some(journal) => journal,
                    None => return,
                },
            };
            let Some(journal) = journal else {
                // A folder scan has no journal to follow.
                return;
            };
            match self.follow(run, letter, journal, emit) {
                watch::Follow::Gap => continue,
                watch::Follow::Stopped => return,
                watch::Follow::Failed(err) => {
                    self.update(run, letter, emit, |d| {
                        d.note = Some(format!("Live updates stopped ({err})"));
                    });
                    return;
                }
            }
        }
    }

    /// Applies the journal to the installed index as it changes. Status goes out at
    /// most once a second; the snapshot is saved every few minutes and on quit.
    fn follow(&self, run: u64, letter: char, journal: Journal, emit: &Emit) -> watch::Follow {
        self.lock_positions().insert(letter, journal);
        let mut last_emit = Instant::now();
        let mut last_save = Instant::now();
        let mut on_batch = |records: &[win32::UsnRecord], position: Journal| -> AppResult<()> {
            let (changed, entries) = {
                let mut volumes = self.write_volumes();
                // Checked under the lock that `start` and `install` also take, so a
                // superseded watcher never touches the index the new run installed.
                if !self.is_current(run) {
                    return Err(super::superseded());
                }
                match volumes.iter_mut().find(|v| root_letter(v) == letter) {
                    Some(index) => {
                        let changed = watch::apply(index, records)?;
                        index.compact_if_wasteful()?;
                        (changed, index.len())
                    }
                    None => (0, 0),
                }
            };
            self.lock_positions().insert(letter, position);
            if changed > 0 {
                self.lock_dirty().insert(letter);
                if last_emit.elapsed() >= STATUS_EVERY {
                    last_emit = Instant::now();
                    self.update(run, letter, emit, |d| {
                        d.entries = entries;
                        d.searchable = entries;
                    });
                }
            }
            if last_save.elapsed() >= SAVE_EVERY && self.lock_dirty().remove(&letter) {
                last_save = Instant::now();
                self.save_installed(letter);
            }
            Ok(())
        };
        watch::follow(letter, journal, &|| self.is_current(run), &mut on_batch)
    }

    /// Reads the drive from scratch and installs it. Returns the journal position to
    /// follow from (`None` inside for a folder scan), or `None` when the build failed
    /// or a newer start superseded it.
    fn build(
        &self,
        run: u64,
        letter: char,
        method: Method,
        emit: &Emit,
    ) -> Option<Option<Journal>> {
        self.update(run, letter, emit, |d| {
            d.state = DriveState::Indexing;
            d.entries = 0;
        });
        let progress = |entries: usize| self.update(run, letter, emit, |d| d.entries = entries);
        let keep_going = || self.is_current(run);
        let hooks = walk::Hooks {
            progress: &progress,
            keep_going: &keep_going,
        };
        let built = match method {
            Method::Mft => match mft::build(letter, &mut |n| progress(n), &keep_going) {
                Ok(built) => Ok((built.index, built.journal, Method::Mft, None)),
                Err(err) => {
                    let note = format!("Folder scan: the file table could not be read ({err})");
                    walk_drive(letter, &hooks).map(|index| (index, None, Method::Walk, Some(note)))
                }
            },
            Method::Walk => {
                walk_drive(letter, &hooks).map(|index| (index, None, Method::Walk, None))
            }
        };
        match built {
            Ok((index, journal, used, note)) => {
                let entries = index.len();
                if !self.is_current(run) {
                    return None;
                }
                // Saved before it is installed, while no lock is held: saving under the
                // read lock would queue the next install, and every search behind it.
                self.save_snapshot(&index, journal);
                if !self.install(run, index) {
                    return None;
                }
                self.update(run, letter, emit, |d| {
                    d.state = DriveState::Ready;
                    d.entries = entries;
                    d.searchable = entries;
                    d.method = used;
                    d.note = note;
                });
                Some(journal)
            }
            Err(err) => {
                self.update(run, letter, emit, |d| {
                    d.state = DriveState::Failed;
                    d.note = Some(err.to_string());
                });
                None
            }
        }
    }

    /// Makes the drive's last snapshot searchable at once. Returns the journal
    /// position it is current to, if it has one. A missing snapshot is the normal
    /// first run; a damaged one is ignored and rebuilt.
    fn load_snapshot(&self, run: u64, letter: char, emit: &Emit) -> Option<Journal> {
        // Restarting with this drive still chosen: the index in memory is newer than
        // its snapshot (live updates since the last save), so keep it and its position.
        let installed = self
            .volumes
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|v| root_letter(v) == letter)
            .map(VolumeIndex::len);
        if let Some(entries) = installed {
            self.update(run, letter, emit, |d| d.searchable = entries);
            return self.lock_positions().get(&letter).copied();
        }
        let path = self.snapshot_path(letter)?;
        match snapshot::load(&path) {
            Ok(loaded) => {
                let entries = loaded.index.len();
                if !self.install(run, loaded.index) {
                    return None;
                }
                self.update(run, letter, emit, |d| d.searchable = entries);
                loaded.journal
            }
            Err(AppError::Io(err)) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => {
                eprintln!("ignoring the snapshot of {letter}: ({err})");
                None
            }
        }
    }

    /// Saves the installed index of `letter` with the journal position it is current
    /// to. It holds the read lock while writing, which is why it runs only every few
    /// minutes and on quit, never after each change.
    fn save_installed(&self, letter: char) {
        let journal = self.lock_positions().get(&letter).copied();
        let volumes = self.volumes.read().unwrap_or_else(PoisonError::into_inner);
        if let Some(index) = volumes.iter().find(|v| root_letter(v) == letter) {
            self.save_snapshot(index, journal);
        }
    }

    fn is_current(&self, run: u64) -> bool {
        self.run.load(Ordering::SeqCst) == run
    }

    /// Saves a drive's index. A failed save only costs the next launch its head
    /// start, so it is logged rather than reported.
    fn save_snapshot(&self, index: &VolumeIndex, journal: Option<Journal>) {
        let letter = root_letter(index);
        let Some(path) = self.snapshot_path(letter) else {
            return;
        };
        // One save at a time: two writers of one drive would share its temp file.
        let _saving = self.saving.lock().unwrap_or_else(PoisonError::into_inner);
        if let Err(err) = snapshot::save(&path, index, journal) {
            eprintln!("could not save the snapshot of {letter}: ({err})");
        }
    }

    fn snapshot_path(&self, letter: char) -> Option<PathBuf> {
        self.snapshots
            .as_deref()
            .map(|dir: &Path| snapshot::path_for(dir, letter))
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

    fn lock_positions(&self) -> MutexGuard<'_, HashMap<char, Journal>> {
        self.positions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_dirty(&self) -> MutexGuard<'_, HashSet<char>> {
        self.dirty.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn walk_drive(letter: char, hooks: &walk::Hooks<'_>) -> AppResult<VolumeIndex> {
    let root = format!("{letter}:\\");
    walk::build(&PathBuf::from(&root), &root, hooks)
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
    fn a_saved_snapshot_is_searchable_in_the_next_service() {
        let dir = std::env::temp_dir().join(format!("findex-service-{}", std::process::id()));
        let first = IndexService::with_snapshots(dir.clone());
        first.save_snapshot(&volume('Q', &["kept.txt"]), None);

        let next = IndexService::with_snapshots(dir.clone());
        assert!(next.search("kept", 10).unwrap().is_empty());
        next.load_snapshot(0, 'Q', &quiet());
        let hits = next.search("kept", 10).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "Q:\\kept.txt");
    }

    #[test]
    fn a_restart_keeps_the_newer_index_in_memory() {
        let dir = std::env::temp_dir().join(format!("findex-restart-{}", std::process::id()));
        let service = IndexService::with_snapshots(dir.clone());
        service.save_snapshot(&volume('Q', &["old.txt"]), None);
        service.install(0, volume('Q', &["new.txt"]));
        service.load_snapshot(0, 'Q', &quiet());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(service.search("new", 10).unwrap().len(), 1);
        assert!(service.search("old", 10).unwrap().is_empty());
    }

    #[test]
    fn a_damaged_snapshot_is_ignored() {
        let dir = std::env::temp_dir().join(format!("findex-damaged-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(snapshot::path_for(&dir, 'Q'), b"FXI1 garbage").unwrap();
        let service = IndexService::with_snapshots(dir.clone());
        service.load_snapshot(0, 'Q', &quiet());
        let _ = std::fs::remove_dir_all(&dir);
        assert!(service.search("garbage", 10).unwrap().is_empty());
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
