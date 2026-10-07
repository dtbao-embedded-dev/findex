//! Builds a volume's index from its NTFS file table in one sequential pass, the way
//! Everything and omni-search do: no folder is opened, so a million names take
//! seconds. Reading the table needs administrator rights.

use crate::error::AppResult;
use crate::model::index::store::VolumeIndex;
use crate::model::win32::{self, Journal};

/// The file reference of an NTFS root folder: record 5, sequence 5.
pub const ROOT_FRN: u64 = 0x0005_0000_0000_0005;

/// The low 48 bits of a file reference are its record number.
const RECORD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

/// Records below 16 are NTFS's own metadata ($MFT, $LogFile, $Extend, ...).
const FIRST_USER_RECORD: u64 = 16;

const ROOT_RECORD: u64 = 5;

/// How often, in entries, the build reports progress.
pub const PROGRESS_EVERY: usize = 65_536;

pub struct Built {
    pub index: VolumeIndex,
    /// Where the change journal stood when the table was read; `None` when the
    /// volume has no active journal, so it cannot be kept current.
    pub journal: Option<Journal>,
}

/// Reads every file and folder name on `letter`'s volume. Stops with
/// [`superseded`](super::superseded) as soon as `keep_going` turns false.
pub fn build(
    letter: char,
    progress: &mut dyn FnMut(usize),
    keep_going: &dyn Fn() -> bool,
) -> AppResult<Built> {
    let volume = win32::open_volume(letter)?;
    let journal = win32::query_journal(&volume).ok();
    // Reading up to the journal's current position lets the watcher resume exactly
    // where this snapshot ends.
    let high_usn = journal.map_or(i64::MAX, |j| j.next_usn);
    let mut index = VolumeIndex::new(format!("{letter}:\\"), ROOT_FRN);
    let mut failed = None;
    win32::enum_mft(&volume, high_usn, |record| {
        if !is_user_entry(record.frn, record.parent) {
            return true;
        }
        if let Err(err) = index.upsert(record.frn, record.parent, &record.name, record.is_dir()) {
            failed = Some(err);
            return false;
        }
        if index.len() % PROGRESS_EVERY == 0 {
            progress(index.len());
            if !keep_going() {
                failed = Some(super::superseded());
                return false;
            }
        }
        true
    })?;
    match failed {
        Some(err) => Err(err),
        None => Ok(Built { index, journal }),
    }
}

/// Leaves out NTFS metadata files and whatever lives inside metadata folders such as
/// `$Extend`, which would otherwise surface as loose entries at the drive root.
pub fn is_user_entry(frn: u64, parent: u64) -> bool {
    let record = frn & RECORD_MASK;
    let parent_record = parent & RECORD_MASK;
    record >= FIRST_USER_RECORD
        && (parent_record >= FIRST_USER_RECORD || parent_record == ROOT_RECORD)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::model::search;

    #[test]
    fn keeps_user_entries_and_drops_ntfs_metadata() {
        assert!(is_user_entry(0x0001_0000_0000_0040, ROOT_FRN));
        assert!(is_user_entry(0x0001_0000_0000_0041, 0x0001_0000_0000_0040));
        // $MFT itself, and $UsnJrnl inside $Extend (record 11).
        assert!(!is_user_entry(0x0001_0000_0000_0000, ROOT_FRN));
        assert!(!is_user_entry(0x0001_0000_0000_0020, 0x000B_0000_0000_000B));
    }

    /// Needs an Administrator terminal:
    /// `cargo test --lib -- --ignored mft_enumerates_system_volume --nocapture`
    #[test]
    #[ignore]
    fn mft_enumerates_system_volume() {
        let start = Instant::now();
        let built = build('C', &mut |_| {}, &|| true).unwrap();
        let took = start.elapsed();
        let count = built.index.len();
        println!(
            "C: {count} entries in {took:?}, journal {:?}",
            built.journal
        );
        assert!(count > 100_000, "only {count} entries");
        let hits = search::run(
            &[built.index],
            &search::parse("notepad.exe"),
            0,
            50,
            &|| false,
        )
        .unwrap()
        .hits;
        let notepad = r"C:\Windows\System32\notepad.exe";
        assert!(
            hits.iter()
                .any(|hit| hit.path.eq_ignore_ascii_case(notepad)),
            "no {notepad} among {:?}",
            hits.iter().map(|hit| &hit.path).collect::<Vec<_>>()
        );
    }
}
