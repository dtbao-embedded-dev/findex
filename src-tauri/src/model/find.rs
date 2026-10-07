//! Answers a search box query with the filter bar's choices: names come from the
//! in-memory index, while a query with tags is answered from the tags table, which
//! the index knows nothing about. A date range is then checked on the disk.

use std::path::Path;

use serde::Serialize;

use crate::error::AppResult;
use crate::model::dated;
use crate::model::db::Db;
use crate::model::index::service::IndexService;
use crate::model::search::{self, Filters, Hit, Query};
use crate::model::tags;

/// What one search answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub hits: Vec<Hit>,
    /// True when a date range was checked on only the best name matches, so more
    /// results may exist for a narrower name.
    pub partial: bool,
}

/// The best `limit` results for `text` narrowed by `filters`. `None` when a newer
/// search replaced this one before it finished.
pub fn search(
    db: &Db,
    index: &IndexService,
    text: &str,
    filters: &Filters,
    limit: usize,
) -> AppResult<Option<Answer>> {
    let mut query = search::parse(text);
    query.narrow(filters);
    let range = filters.modified();
    // A date rejects most name matches, so more of them are needed to fill a page.
    let pool = if range.is_some() {
        dated::pool_for(limit).max(limit)
    } else {
        limit
    };
    let cancelled = index.begin_search();
    let hits = if query.tags().is_empty() {
        match index.search(&query, pool, &cancelled) {
            Some(hits) => hits,
            None => return Ok(None),
        }
    } else {
        tagged(db, &query, pool)?
    };
    let Some(range) = range else {
        return Ok(Some(Answer {
            hits,
            partial: false,
        }));
    };
    let pool_full = hits.len() >= pool;
    Ok(
        dated::keep(hits, &range, limit, &cancelled).map(|hits| Answer {
            partial: pool_full && hits.len() < limit,
            hits,
        }),
    )
}

/// The tagged folders that still exist and match the query's name terms, best first.
fn tagged(db: &Db, query: &Query, limit: usize) -> AppResult<Vec<Hit>> {
    let mut ranked: Vec<(u32, Hit)> = tags::folders_with(db, query.tags())?
        .into_iter()
        .filter(|path| Path::new(path).is_dir())
        .filter_map(|path| {
            // A drive root has no name of its own; it is shown by its path.
            let name = Path::new(&path)
                .file_name()
                .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
            let rank = search::rank_name(query, &name, true)?;
            Some((
                rank,
                Hit {
                    name,
                    path,
                    is_dir: true,
                },
            ))
        })
        .collect();
    // Best match first, then by name.
    ranked.sort_by(|(a_rank, a), (b_rank, b)| {
        b_rank
            .cmp(a_rank)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(ranked.into_iter().take(limit).map(|(_, hit)| hit).collect())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::model::db;
    use crate::model::index::store::VolumeIndex;
    use crate::model::search::Kind;

    fn folder(name: &str) -> String {
        let dir = std::env::temp_dir()
            .join(format!("findex-find-{}", std::process::id()))
            .join(name);
        std::fs::create_dir_all(&dir).unwrap();
        dir.display().to_string()
    }

    fn names(answer: AppResult<Option<Answer>>) -> Vec<String> {
        answer
            .unwrap()
            .unwrap()
            .hits
            .into_iter()
            .map(|hit| hit.name)
            .collect()
    }

    fn typed(db: &Db, index: &IndexService, text: &str, limit: usize) -> AppResult<Option<Answer>> {
        search(db, index, text, &Filters::default(), limit)
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    /// A drive index rooted at a real folder, so the dates of its files can be read.
    fn rooted(dir: &Path, names: &[&str]) -> IndexService {
        let mut volume = VolumeIndex::new(format!("{}\\", dir.display()), 0);
        for (i, name) in names.iter().enumerate() {
            volume.upsert(i as u64 + 1, 0, name, false).unwrap();
        }
        IndexService::with_volumes(vec![volume])
    }

    fn touch(path: &Path, ms: i64) {
        std::fs::write(path, b"x").unwrap();
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_millis(ms as u64))
            .unwrap();
    }

    #[test]
    fn a_query_without_tags_goes_to_the_index() {
        let db = db::open_in_memory().unwrap();
        tags::add(&db, &folder("proj-index"), "work").unwrap();
        let index = IndexService::default();
        assert_eq!(
            typed(&db, &index, "proj", 10).unwrap(),
            Some(Answer {
                hits: Vec::new(),
                partial: false
            })
        );
    }

    #[test]
    fn a_tag_query_lists_tagged_folders_narrowed_by_name() {
        let db = db::open_in_memory().unwrap();
        let index = IndexService::default();
        let (a, b, c) = (folder("proj-alpha"), folder("notes"), folder("proj-beta"));
        tags::add(&db, &a, "work").unwrap();
        tags::add(&db, &b, "work").unwrap();
        tags::add(&db, &c, "fw").unwrap();

        assert_eq!(
            names(typed(&db, &index, "tag:work", 10)),
            ["notes", "proj-alpha"]
        );
        assert_eq!(
            names(typed(&db, &index, "proj tag:work", 10)),
            ["proj-alpha"]
        );
        assert_eq!(
            names(typed(&db, &index, "tag:work;fw", 10)),
            Vec::<String>::new()
        );
        assert_eq!(names(typed(&db, &index, "tag:work", 1)).len(), 1);

        let hits = typed(&db, &index, "tag:fw", 10).unwrap().unwrap().hits;
        assert_eq!(hits[0].path, c);
        assert!(hits[0].is_dir);
    }

    #[test]
    fn a_tagged_folder_deleted_since_is_left_out() {
        let db = db::open_in_memory().unwrap();
        let index = IndexService::default();
        let gone = folder("gone");
        tags::add(&db, &gone, "old").unwrap();
        std::fs::remove_dir_all(&gone).unwrap();
        assert!(names(typed(&db, &index, "tag:old", 10)).is_empty());
    }

    #[test]
    fn filter_tags_and_kind_narrow_like_typed_ones() {
        let db = db::open_in_memory().unwrap();
        let index = IndexService::default();
        let (a, b) = (folder("flt-alpha"), folder("flt-beta"));
        tags::add(&db, &a, "work").unwrap();
        tags::add(&db, &b, "home").unwrap();
        let by_tag = Filters {
            tags: vec!["work".into()],
            ..Filters::default()
        };
        assert_eq!(names(search(&db, &index, "", &by_tag, 10)), ["flt-alpha"]);
        let files_only = Filters {
            kind: Kind::File,
            ..by_tag
        };
        assert!(names(search(&db, &index, "", &files_only, 10)).is_empty());
    }

    #[test]
    fn a_date_range_keeps_what_changed_inside_it() {
        let dir = std::env::temp_dir().join(format!("findex-find-dated-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let day = 86_400_000;
        let now = now_ms();
        touch(&dir.join("report-old.txt"), now - 40 * day);
        touch(&dir.join("report-new.txt"), now - day);
        let index = rooted(
            &dir,
            &["report-old.txt", "report-new.txt", "report-gone.txt"],
        );
        let db = db::open_in_memory().unwrap();
        let last_month = Filters {
            modified_from: Some(now - 30 * day),
            ..Filters::default()
        };
        let answer = search(&db, &index, "report", &last_month, 10)
            .unwrap()
            .unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            answer
                .hits
                .iter()
                .map(|h| h.name.as_str())
                .collect::<Vec<_>>(),
            ["report-new.txt"]
        );
        assert!(!answer.partial);
    }

    #[test]
    fn a_dated_search_says_when_only_the_best_matches_were_checked() {
        let dir = std::env::temp_dir().join(format!("findex-find-partial-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // More name matches than the pool holds, none of them on the disk.
        let names: Vec<String> = (0..dated::pool_for(10) + 1)
            .map(|i| format!("log{i}.txt"))
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let index = rooted(&dir, &refs);
        let db = db::open_in_memory().unwrap();
        let recent = Filters {
            modified_from: Some(0),
            ..Filters::default()
        };
        let answer = search(&db, &index, "log", &recent, 10).unwrap().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(answer.hits.is_empty());
        assert!(answer.partial);
    }

    #[test]
    fn a_date_range_narrows_tagged_folders_too() {
        let db = db::open_in_memory().unwrap();
        let index = IndexService::default();
        tags::add(&db, &folder("dated-tagged"), "dated").unwrap();
        let tagged = |from: Option<i64>, to: Option<i64>| Filters {
            tags: vec!["dated".into()],
            modified_from: from,
            modified_to: to,
            ..Filters::default()
        };
        let hour = 3_600_000;
        assert_eq!(
            names(search(
                &db,
                &index,
                "",
                &tagged(Some(now_ms() - hour), None),
                10
            )),
            ["dated-tagged"]
        );
        assert!(names(search(&db, &index, "", &tagged(None, Some(1_000)), 10)).is_empty());
    }

    #[test]
    fn a_newer_search_cancels_an_older_one() {
        let index = IndexService::default();
        let older = index.begin_search();
        assert!(!older());
        let newer = index.begin_search();
        assert!(older());
        assert!(!newer());
    }
}
