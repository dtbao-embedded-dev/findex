//! Which drives exist, how each one can be indexed, and which ones the user chose.
//! The choice is stored in the `setting` table as a JSON array of drive letters.

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::model::db::Db;
use crate::model::win32::{self, Volume};

/// The setting that holds the chosen drives; absent until setup is done once.
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

/// The chosen drive letters, or `None` before setup has been done.
pub fn load_selection(db: &Db) -> AppResult<Option<Vec<String>>> {
    let Some(json) = db.get_setting(SCAN_DRIVES_KEY)? else {
        return Ok(None);
    };
    let letters: Vec<String> = serde_json::from_str(&json)
        .map_err(|_| AppError::Invalid("the saved drive list is unreadable".into()))?;
    Ok(Some(letters))
}

/// Validates and stores the chosen letters: upper case, one each, in order.
pub fn save_selection(db: &Db, letters: &[String]) -> AppResult<Vec<String>> {
    let clean = normalize(letters)?;
    let json = serde_json::to_string(&clean)
        .map_err(|_| AppError::Invalid("the drive list could not be saved".into()))?;
    db.set_setting(SCAN_DRIVES_KEY, &json)?;
    Ok(clean)
}

fn normalize(letters: &[String]) -> AppResult<Vec<String>> {
    let mut clean = Vec::new();
    for letter in letters {
        let mut chars = letter.trim().trim_end_matches([':', '\\']).chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if c.is_ascii_alphabetic() => {
                clean.push(c.to_ascii_uppercase().to_string());
            }
            _ => {
                return Err(AppError::Invalid(format!(
                    "\"{letter}\" is not a drive letter"
                )))
            }
        }
    }
    clean.sort();
    clean.dedup();
    if clean.is_empty() {
        return Err(AppError::Invalid("choose at least one drive".into()));
    }
    Ok(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

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

    #[test]
    fn the_selection_is_absent_until_saved_then_round_trips() {
        let db = crate::model::db::open_in_memory().unwrap();
        assert_eq!(load_selection(&db).unwrap(), None);
        let saved = save_selection(&db, &strings(&["d", "C:", "c:\\"])).unwrap();
        assert_eq!(saved, ["C", "D"]);
        assert_eq!(load_selection(&db).unwrap(), Some(strings(&["C", "D"])));
    }

    #[test]
    fn refuses_an_empty_or_invalid_selection() {
        let db = crate::model::db::open_in_memory().unwrap();
        assert!(matches!(
            save_selection(&db, &[]),
            Err(AppError::Invalid(_))
        ));
        assert!(matches!(
            save_selection(&db, &strings(&["CD"])),
            Err(AppError::Invalid(_))
        ));
        assert!(matches!(
            save_selection(&db, &strings(&["1"])),
            Err(AppError::Invalid(_))
        ));
        assert_eq!(load_selection(&db).unwrap(), None);
    }
}
