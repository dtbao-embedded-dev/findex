//! Keeps an NTFS volume's index current from its change journal, the way
//! omni-search does: poll the journal from where the index stops, apply each
//! create, delete, rename and move, and ask for a full rebuild when the journal no
//! longer reaches back that far.

use std::time::Duration;

use crate::error::{AppError, AppResult};
use crate::model::index::mft;
use crate::model::index::store::VolumeIndex;
use crate::model::win32::{self, Journal, UsnRecord};

const REASON_FILE_CREATE: u32 = 0x0000_0100;
const REASON_FILE_DELETE: u32 = 0x0000_0200;
const REASON_RENAME_NEW_NAME: u32 = 0x0000_2000;

/// How long to wait after a read that found nothing new.
const IDLE_POLL: Duration = Duration::from_millis(250);

/// Bytes per journal read.
const READ_BUFFER: usize = 64 * 1024;

/// Applies journal records in order; returns how many changed the index.
///
/// A delete removes the entry. A create, or the new-name half of a rename or move,
/// adds or updates it. Any other change to a file the index does not know yet adds
/// it too, so an entry missed earlier still appears; other changes (writes,
/// attribute edits, the old-name half of a rename) leave the index as it is.
pub fn apply(index: &mut VolumeIndex, records: &[UsnRecord]) -> AppResult<usize> {
    let mut changed = 0;
    for record in records {
        if record.reason & REASON_FILE_DELETE != 0 {
            changed += usize::from(index.remove(record.frn));
            continue;
        }
        let names_it = record.reason & (REASON_FILE_CREATE | REASON_RENAME_NEW_NAME) != 0;
        let unknown = index.node_of(record.frn).is_none();
        if !(names_it || unknown) || !mft::is_user_entry(record.frn, record.parent) {
            continue;
        }
        let before = index.node_of(record.frn).map(|id| {
            (
                index.parent(id),
                index.name(id).to_owned(),
                index.is_dir(id),
            )
        });
        index.upsert(record.frn, record.parent, &record.name, record.is_dir())?;
        let after = Some((record.parent, record.name.clone(), record.is_dir()));
        changed += usize::from(before != after);
    }
    Ok(changed)
}

/// Why following a journal ended.
pub enum Follow {
    /// The caller stopped it: a newer start replaced this drive's work.
    Stopped,
    /// The journal no longer reaches back to the index; rebuild from the file table.
    Gap,
    Failed(AppError),
}

/// Reads the journal of `letter` from `journal.next_usn` on, handing every batch of
/// changes and the position after it to `on_batch`, until `keep_going` says stop.
pub fn follow(
    letter: char,
    journal: Journal,
    keep_going: &dyn Fn() -> bool,
    on_batch: &mut dyn FnMut(&[UsnRecord], Journal) -> AppResult<()>,
) -> Follow {
    let volume = match win32::open_volume(letter) {
        Ok(volume) => volume,
        Err(err) => return Follow::Failed(err.into()),
    };
    let mut position = journal;
    let mut buffer = vec![0_u64; READ_BUFFER / 8];
    while keep_going() {
        match win32::read_usn(&volume, position.id, position.next_usn, &mut buffer) {
            Ok(batch) => {
                position.next_usn = batch.next_usn;
                if batch.records.is_empty() {
                    std::thread::sleep(IDLE_POLL);
                } else if let Err(err) = on_batch(&batch.records, position) {
                    return Follow::Failed(err);
                }
            }
            Err(err) if win32::is_journal_gap(&err) => return Follow::Gap,
            Err(err) => return Follow::Failed(err.into()),
        }
    }
    Follow::Stopped
}

/// True when `letter`'s journal still holds every change since `saved`, so an index
/// current to `saved` can be brought up to date instead of rebuilt.
pub fn continues(letter: char, saved: &Journal) -> bool {
    let Ok(volume) = win32::open_volume(letter) else {
        return false;
    };
    match win32::query_journal(&volume) {
        Ok(now) => {
            now.id == saved.id && saved.next_usn >= now.first_usn && saved.next_usn <= now.next_usn
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::search;

    const ROOT: u64 = mft::ROOT_FRN;
    const DOCS: u64 = 0x0001_0000_0000_0040;
    const ARCHIVE: u64 = 0x0001_0000_0000_0041;
    const FILE: u64 = 0x0002_0000_0000_0050;

    fn record(frn: u64, parent: u64, reason: u32, name: &str, dir: bool) -> UsnRecord {
        UsnRecord {
            frn,
            parent,
            usn: 0,
            reason,
            attributes: if dir { 0x10 } else { 0x20 },
            name: name.into(),
        }
    }

    fn index() -> VolumeIndex {
        let mut index = VolumeIndex::new("C:\\", ROOT);
        index.upsert(DOCS, ROOT, "Docs", true).unwrap();
        index.upsert(ARCHIVE, ROOT, "Archive", true).unwrap();
        index
    }

    fn paths(index: VolumeIndex, text: &str) -> Vec<String> {
        search::run(&[index], &search::parse(text), 10, &|| false)
            .unwrap()
            .into_iter()
            .map(|hit| hit.path)
            .collect()
    }

    #[test]
    fn a_created_file_appears_with_its_path() {
        let mut index = index();
        let changed = apply(
            &mut index,
            &[record(FILE, DOCS, REASON_FILE_CREATE, "plan.md", false)],
        )
        .unwrap();
        assert_eq!(changed, 1);
        assert_eq!(paths(index, "plan"), ["C:\\Docs\\plan.md"]);
    }

    #[test]
    fn a_rename_replaces_the_old_name() {
        let mut index = index();
        apply(
            &mut index,
            &[
                record(FILE, DOCS, REASON_FILE_CREATE, "draft.md", false),
                record(FILE, DOCS, 0x1000, "draft.md", false),
                record(FILE, DOCS, REASON_RENAME_NEW_NAME, "final.md", false),
            ],
        )
        .unwrap();
        assert!(paths(index_with(&index), "draft").is_empty());
        assert_eq!(paths(index, "final"), ["C:\\Docs\\final.md"]);
    }

    #[test]
    fn a_move_changes_the_folder() {
        let mut index = index();
        apply(
            &mut index,
            &[record(FILE, DOCS, REASON_FILE_CREATE, "a.txt", false)],
        )
        .unwrap();
        apply(
            &mut index,
            &[record(
                FILE,
                ARCHIVE,
                REASON_RENAME_NEW_NAME,
                "a.txt",
                false,
            )],
        )
        .unwrap();
        assert_eq!(paths(index, "a.txt"), ["C:\\Archive\\a.txt"]);
    }

    #[test]
    fn a_renamed_folder_carries_its_contents() {
        let mut index = index();
        apply(
            &mut index,
            &[record(FILE, DOCS, REASON_FILE_CREATE, "spec.md", false)],
        )
        .unwrap();
        apply(
            &mut index,
            &[record(DOCS, ROOT, REASON_RENAME_NEW_NAME, "Specs", true)],
        )
        .unwrap();
        assert_eq!(paths(index, "spec.md"), ["C:\\Specs\\spec.md"]);
    }

    #[test]
    fn a_delete_removes_the_entry() {
        let mut index = index();
        apply(
            &mut index,
            &[record(FILE, DOCS, REASON_FILE_CREATE, "gone.txt", false)],
        )
        .unwrap();
        let changed = apply(
            &mut index,
            &[record(
                FILE,
                DOCS,
                REASON_FILE_DELETE | 0x8000_0000,
                "gone.txt",
                false,
            )],
        )
        .unwrap();
        assert_eq!(changed, 1);
        assert!(paths(index, "gone").is_empty());
    }

    #[test]
    fn writes_to_a_known_file_change_nothing() {
        let mut index = index();
        apply(
            &mut index,
            &[record(FILE, DOCS, REASON_FILE_CREATE, "log.txt", false)],
        )
        .unwrap();
        let data_extend = 0x0000_0002;
        let changed = apply(
            &mut index,
            &vec![record(FILE, DOCS, data_extend, "log.txt", false); 5],
        )
        .unwrap();
        assert_eq!(changed, 0);
    }

    #[test]
    fn a_change_to_an_unknown_file_adds_it() {
        let mut index = index();
        apply(
            &mut index,
            &[record(FILE, DOCS, 0x0000_0002, "missed.txt", false)],
        )
        .unwrap();
        assert_eq!(paths(index, "missed"), ["C:\\Docs\\missed.txt"]);
    }

    #[test]
    fn ntfs_metadata_changes_are_ignored() {
        let mut index = index();
        let usn_journal = 0x0001_0000_0000_0020;
        let extend = 0x000B_0000_0000_000B;
        apply(
            &mut index,
            &[record(usn_journal, extend, REASON_FILE_CREATE, "$J", false)],
        )
        .unwrap();
        assert!(paths(index, "$j").is_empty());
    }

    /// The same entries as `index`, rebuilt, so one can be searched and kept.
    fn index_with(source: &VolumeIndex) -> VolumeIndex {
        let mut copy = VolumeIndex::new(source.root(), source.root_frn());
        for id in (0..source.node_count()).filter(|&id| source.is_live(id)) {
            copy.upsert(
                source.frn(id),
                source.parent(id),
                source.name(id),
                source.is_dir(id),
            )
            .unwrap();
        }
        copy
    }

    /// Needs an Administrator terminal, on a machine whose temp folder is on C::
    /// `cargo test --lib -- --ignored usn_sees_new_file --nocapture`
    #[test]
    #[ignore]
    fn usn_sees_new_file() {
        use std::sync::mpsc;
        use std::time::Instant;

        let volume = win32::open_volume('C').unwrap();
        let journal = win32::query_journal(&volume).unwrap();
        let (tx, rx) = mpsc::channel::<Vec<UsnRecord>>();
        // Runs until a send fails, which is how it ends once the test drops `rx`.
        let watcher = std::thread::spawn(move || {
            follow('C', journal, &|| true, &mut |batch, _| {
                tx.send(batch.to_vec())
                    .map_err(|_| AppError::Invalid("test ended".into()))
            })
        });
        let mut index = VolumeIndex::new("C:\\", ROOT);
        let dir = std::env::temp_dir();
        let unique = format!("findex-usn-{}", std::process::id());
        let first = dir.join(format!("{unique}-a.txt"));
        let second = dir.join(format!("{unique}-b.txt"));
        let wait_for = |index: &mut VolumeIndex, text: &str, present: bool| {
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(2) {
                while let Ok(batch) = rx.try_recv() {
                    apply(index, &batch).unwrap();
                }
                let found = !paths(index_with(index), text).is_empty();
                if found == present {
                    return start.elapsed();
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            panic!(
                "{text} {} within 2 s",
                if present {
                    "did not appear"
                } else {
                    "did not go"
                }
            );
        };
        std::fs::write(&first, "x").unwrap();
        println!(
            "create seen in {:?}",
            wait_for(&mut index, &format!("{unique}-a"), true)
        );
        std::fs::rename(&first, &second).unwrap();
        println!(
            "rename seen in {:?}",
            wait_for(&mut index, &format!("{unique}-b"), true)
        );
        assert!(paths(index_with(&index), &format!("{unique}-a")).is_empty());
        std::fs::remove_file(&second).unwrap();
        println!(
            "delete seen in {:?}",
            wait_for(&mut index, &format!("{unique}-b"), false)
        );
        drop(rx);
        // The watcher ends at its next batch, when sending fails.
        std::fs::write(dir.join(format!("{unique}-end.txt")), "x").unwrap();
        let _ = std::fs::remove_file(dir.join(format!("{unique}-end.txt")));
        assert!(matches!(watcher.join().unwrap(), Follow::Failed(_)));
    }
}
