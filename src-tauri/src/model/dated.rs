//! Narrows results to what last changed within a time range. The index holds names
//! only, so dates are read from the disk: the name matches are checked in rank order,
//! a batch at a time in parallel, until enough fall inside the range.

use rayon::prelude::*;

use crate::model::details;
use crate::model::search::Hit;

/// A span of last-change times, in milliseconds since the Unix epoch: `from` is
/// included, `to` is not. A missing bound leaves that side open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub from: Option<i64>,
    pub to: Option<i64>,
}

impl Range {
    /// A range when at least one bound is set; with none, nothing is narrowed.
    pub fn new(from: Option<i64>, to: Option<i64>) -> Option<Self> {
        (from.is_some() || to.is_some()).then_some(Self { from, to })
    }

    pub fn contains(&self, ms: i64) -> bool {
        self.from.map_or(true, |from| ms >= from) && self.to.map_or(true, |to| ms < to)
    }
}

/// How many name matches a dated search may check for `limit` results. A date
/// rejects most matches, so the pool is ten times the page, within bounds that keep
/// a search well under a second on a warm disk.
pub fn pool_for(limit: usize) -> usize {
    limit.saturating_mul(10).clamp(20_000, 100_000)
}

/// Paths whose dates are read at once; small enough to stop soon after `limit`.
const BATCH: usize = 1024;

/// The hits, in the order given, whose last change falls inside `range`, up to
/// `limit`. A path that can no longer be read is left out. `None` when `cancelled`
/// reports that a newer search replaced this one.
pub fn keep(
    hits: Vec<Hit>,
    range: &Range,
    limit: usize,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Option<Vec<Hit>> {
    let mut kept = Vec::new();
    let mut rest = hits.into_iter();
    while kept.len() < limit {
        let batch: Vec<Hit> = rest.by_ref().take(BATCH).collect();
        if batch.is_empty() {
            break;
        }
        if cancelled() {
            return None;
        }
        // Reading dates waits on the disk, so the reads overlap; order is kept.
        let inside: Vec<bool> = batch
            .par_iter()
            .map(|hit| {
                std::fs::metadata(&hit.path)
                    .ok()
                    .and_then(|meta| details::modified_ms(&meta))
                    .is_some_and(|ms| range.contains(ms))
            })
            .collect();
        kept.extend(
            batch
                .into_iter()
                .zip(inside)
                .filter_map(|(hit, inside)| inside.then_some(hit)),
        );
    }
    kept.truncate(limit);
    Some(kept)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;

    const DAY: i64 = 86_400_000;
    /// 2023-11-14T22:13:20Z, a fixed moment the files are dated around.
    const BASE: i64 = 1_700_000_000_000;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("findex-dated-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A file last changed `days` after `BASE`.
    fn dated(dir: &std::path::Path, name: &str, days: i64) -> Hit {
        let path = dir.join(name);
        std::fs::write(&path, b"x").unwrap();
        let when = UNIX_EPOCH + Duration::from_millis((BASE + days * DAY) as u64);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(when)
            .unwrap();
        Hit {
            name: name.to_owned(),
            path: path.display().to_string(),
            is_dir: false,
        }
    }

    fn names(hits: Option<Vec<Hit>>) -> Vec<String> {
        hits.unwrap().into_iter().map(|hit| hit.name).collect()
    }

    #[test]
    fn a_range_includes_its_start_and_excludes_its_end() {
        let range = Range::new(Some(10), Some(20)).unwrap();
        assert!(range.contains(10));
        assert!(range.contains(19));
        assert!(!range.contains(20));
        assert!(!range.contains(9));
        assert!(Range::new(None, Some(5)).unwrap().contains(i64::MIN));
        assert!(Range::new(Some(5), None).unwrap().contains(i64::MAX));
        assert_eq!(Range::new(None, None), None);
    }

    #[test]
    fn the_pool_grows_with_the_page_within_bounds() {
        assert_eq!(pool_for(200), 20_000);
        assert_eq!(pool_for(5_000), 50_000);
        assert_eq!(pool_for(10_000), 100_000);
        assert_eq!(pool_for(usize::MAX), 100_000);
    }

    #[test]
    fn keeps_only_what_changed_inside_the_range_in_rank_order() {
        let dir = scratch("keep");
        let hits = vec![
            dated(&dir, "c-inside.txt", 3),
            dated(&dir, "a-before.txt", -2),
            dated(&dir, "b-inside.txt", 0),
            dated(&dir, "d-after.txt", 7),
        ];
        let range = Range::new(Some(BASE), Some(BASE + 7 * DAY)).unwrap();
        let kept = keep(hits, &range, 10, &|| false);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(names(kept), ["c-inside.txt", "b-inside.txt"]);
    }

    #[test]
    fn stops_at_the_limit_and_leaves_out_missing_paths() {
        let dir = scratch("limit");
        let mut hits = vec![Hit {
            name: "gone.txt".into(),
            path: dir.join("gone.txt").display().to_string(),
            is_dir: false,
        }];
        hits.extend((0..5).map(|i| dated(&dir, &format!("f{i}.txt"), 1)));
        let range = Range::new(Some(BASE), None).unwrap();
        let kept = keep(hits, &range, 3, &|| false);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(names(kept), ["f0.txt", "f1.txt", "f2.txt"]);
    }

    #[test]
    fn a_cancelled_check_returns_none() {
        let dir = scratch("cancel");
        let hits = vec![dated(&dir, "a.txt", 0)];
        let range = Range::new(Some(BASE), None).unwrap();
        let kept = keep(hits, &range, 10, &|| true);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(kept.is_none());
    }
}
