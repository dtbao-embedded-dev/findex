//! Settings that carry a rule. Each is stored as text in the `setting` table and
//! checked here on the way in and again on the way out, so a value written some
//! other way never reaches the code that uses it.

use crate::error::{AppError, AppResult};
use crate::model::db::Db;
use crate::model::search::DEFAULT_LIMIT;

const RESULTS_KEY: &str = "search.results_limit";

/// The fewest results a search may be set to show.
pub const RESULTS_MIN: usize = 200;

/// The most results a search may be set to show; beyond this a page stops being
/// read and every keystroke pays for rows nobody scrolls to.
pub const RESULTS_MAX: usize = 10_000;

/// How many results a search shows: the saved value, or the default when none was
/// saved or the saved one is out of range.
pub fn results_limit(db: &Db) -> AppResult<usize> {
    Ok(db
        .get_setting(RESULTS_KEY)?
        .and_then(|text| text.parse().ok())
        .filter(|limit| (RESULTS_MIN..=RESULTS_MAX).contains(limit))
        .unwrap_or(DEFAULT_LIMIT))
}

/// Saves how many results a search shows and returns it. A value out of range is
/// refused and the saved one is kept.
pub fn set_results_limit(db: &Db, limit: usize) -> AppResult<usize> {
    if !(RESULTS_MIN..=RESULTS_MAX).contains(&limit) {
        return Err(AppError::Invalid(format!(
            "results per search must be between {RESULTS_MIN} and {RESULTS_MAX}"
        )));
    }
    db.set_setting(RESULTS_KEY, &limit.to_string())?;
    Ok(limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::db;

    #[test]
    fn the_limit_defaults_to_200() {
        let db = db::open_in_memory().unwrap();
        assert_eq!(results_limit(&db).unwrap(), 200);
    }

    #[test]
    fn a_saved_limit_is_read_back() {
        let db = db::open_in_memory().unwrap();
        assert_eq!(set_results_limit(&db, 5_000).unwrap(), 5_000);
        assert_eq!(results_limit(&db).unwrap(), 5_000);
        assert_eq!(set_results_limit(&db, RESULTS_MIN).unwrap(), 200);
        assert_eq!(set_results_limit(&db, RESULTS_MAX).unwrap(), 10_000);
        assert_eq!(results_limit(&db).unwrap(), 10_000);
    }

    #[test]
    fn a_limit_out_of_range_is_refused_and_the_old_one_kept() {
        let db = db::open_in_memory().unwrap();
        set_results_limit(&db, 1_000).unwrap();
        for bad in [0, 199, 10_001, usize::MAX] {
            assert!(
                matches!(set_results_limit(&db, bad), Err(AppError::Invalid(_))),
                "{bad}"
            );
        }
        assert_eq!(results_limit(&db).unwrap(), 1_000);
    }

    #[test]
    fn a_value_stored_some_other_way_falls_back_to_the_default() {
        let db = db::open_in_memory().unwrap();
        for stored in ["abc", "50", "99999", ""] {
            db.set_setting(RESULTS_KEY, stored).unwrap();
            assert_eq!(results_limit(&db).unwrap(), 200, "{stored}");
        }
    }
}
