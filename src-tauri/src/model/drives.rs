//! Which drives exist, how each one can be indexed, and which one is active: the
//! app indexes and searches one drive at a time. The choice is stored in the
//! `setting` table as a JSON array holding that one letter.

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::model::db::Db;
use crate::model::win32::{self, Volume};

/// The setting that holds the active drive; absent until the first launch picks one.
pub const SCAN_DRIVES_KEY: &str = "scan.drives";

/// How a drive's names are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    /// The NTFS file table, read in one pass, then kept current from the USN journal.
    Mft,
    /// A walk over every folder; refreshed only when indexed again.
    Walk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    pub letter: String,
    pub label: String,
    pub filesystem: String,
    pub removable: bool,
    pub total: u64,
    pub free: u64,
    pub method: Method,
}

/// Every local drive with media, in letter order.
pub fn list() -> Vec<Drive> {
    win32::list_volumes().into_iter().map(describe).collect()
}

/// Only NTFS keeps the file table and journal the fast path reads.
pub fn method_for(filesystem: &str) -> Method {
    if filesystem.eq_ignore_ascii_case("NTFS") {
        Method::Mft
    } else {
        Method::Walk
    }
}

fn describe(volume: Volume) -> Drive {
    Drive {
        letter: volume.letter.to_string(),
        method: method_for(&volume.filesystem),
        label: volume.label,
        filesystem: volume.filesystem,
        removable: volume.removable,
        total: volume.total,
        free: volume.free,
    }
}

/// The drive being indexed and searched, or `None` before one was ever chosen. An
/// older version stored several drives; only the first of them stays active.
pub fn load_active(db: &Db) -> AppResult<Option<String>> {
    let Some(json) = db.get_setting(SCAN_DRIVES_KEY)? else {
        return Ok(None);
    };
    let letters: Vec<String> = serde_json::from_str(&json)
        .map_err(|_| AppError::Invalid("the saved drive is unreadable".into()))?;
    Ok(letters.into_iter().next())
}

/// Validates and stores the one active drive, upper case. Stored as a one-item list,
/// the format older versions read and wrote.
pub fn save_active(db: &Db, letter: &str) -> AppResult<String> {
    let clean = normalize(letter)?;
    let json = serde_json::to_string(&[&clean])
        .map_err(|_| AppError::Invalid("the drive could not be saved".into()))?;
    db.set_setting(SCAN_DRIVES_KEY, &json)?;
    Ok(clean)
}

/// The active drive; on the first launch, the system drive is chosen and saved, so
/// the app opens straight on search. `None` only when no drive can be found.
pub fn ensure_active(db: &Db) -> AppResult<Option<String>> {
    if let Some(letter) = load_active(db)? {
        return Ok(Some(letter));
    }
    let system = std::env::var("SystemDrive").ok();
    // The system drive always exists; listing drives can wait on a sleeping USB disk.
    let pick = match system.as_deref().map(normalize) {
        Some(Ok(letter)) => Some(letter),
        _ => default_letter(&list()),
    };
    pick.map(|letter| save_active(db, &letter)).transpose()
}

/// The drive to start with when the system drive is unknown: the first fixed NTFS
/// drive, which indexes fastest, otherwise the first drive.
pub fn default_letter(drives: &[Drive]) -> Option<String> {
    drives
        .iter()
        .find(|d| d.method == Method::Mft && !d.removable)
        .or_else(|| drives.first())
        .map(|d| d.letter.clone())
}

fn normalize(letter: &str) -> AppResult<String> {
    let mut chars = letter.trim().trim_end_matches([':', '\\']).chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => Ok(c.to_ascii_uppercase().to_string()),
        _ => Err(AppError::Invalid(format!(
            "\"{letter}\" is not a drive letter"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntfs_uses_the_file_table_and_others_a_walk() {
        assert_eq!(method_for("NTFS"), Method::Mft);
        assert_eq!(method_for("exFAT"), Method::Walk);
        assert_eq!(method_for("FAT32"), Method::Walk);
    }

    #[test]
    fn lists_the_system_drive_as_mft() {
        let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let drives = list();
        let drive = drives
            .iter()
            .find(|d| system.starts_with(&d.letter))
            .unwrap();
        assert_eq!(drive.method, Method::Mft);
        assert!(drive.total > 0);
    }

    fn drive(letter: &str, method: Method, removable: bool) -> Drive {
        Drive {
            letter: letter.into(),
            label: String::new(),
            filesystem: String::new(),
            removable,
            total: 0,
            free: 0,
            method,
        }
    }

    #[test]
    fn the_active_drive_is_absent_until_saved_then_round_trips() {
        let db = crate::model::db::open_in_memory().unwrap();
        assert_eq!(load_active(&db).unwrap(), None);
        assert_eq!(save_active(&db, "d:\\").unwrap(), "D");
        assert_eq!(load_active(&db).unwrap().as_deref(), Some("D"));
        assert_eq!(save_active(&db, "e").unwrap(), "E");
        assert_eq!(load_active(&db).unwrap().as_deref(), Some("E"));
    }

    #[test]
    fn a_saved_list_of_several_drives_keeps_only_the_first() {
        let db = crate::model::db::open_in_memory().unwrap();
        db.set_setting(SCAN_DRIVES_KEY, r#"["C","E"]"#).unwrap();
        assert_eq!(load_active(&db).unwrap().as_deref(), Some("C"));
    }

    #[test]
    fn refuses_an_invalid_drive_letter() {
        let db = crate::model::db::open_in_memory().unwrap();
        for bad in ["", "CD", "1"] {
            assert!(matches!(save_active(&db, bad), Err(AppError::Invalid(_))));
        }
        assert_eq!(load_active(&db).unwrap(), None);
    }

    #[test]
    fn the_first_launch_picks_and_saves_the_system_drive() {
        let db = crate::model::db::open_in_memory().unwrap();
        let system = std::env::var("SystemDrive").unwrap();
        let expected = system[..1].to_ascii_uppercase();
        assert_eq!(ensure_active(&db).unwrap(), Some(expected.clone()));
        assert_eq!(load_active(&db).unwrap(), Some(expected));
    }

    #[test]
    fn a_saved_drive_is_kept_over_the_system_drive() {
        let db = crate::model::db::open_in_memory().unwrap();
        save_active(&db, "Q").unwrap();
        assert_eq!(ensure_active(&db).unwrap().as_deref(), Some("Q"));
    }

    #[test]
    fn the_default_prefers_a_fixed_ntfs_drive_then_the_first() {
        let usb = drive("D", Method::Mft, true);
        let fat = drive("E", Method::Walk, false);
        let ntfs = drive("F", Method::Mft, false);
        assert_eq!(
            default_letter(&[usb.clone(), fat.clone(), ntfs]).as_deref(),
            Some("F")
        );
        assert_eq!(default_letter(&[fat, usb]).as_deref(), Some("E"));
        assert_eq!(default_letter(&[]), None);
    }
}
