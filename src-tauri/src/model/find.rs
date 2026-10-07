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

/// What one search answers: a page of results and what is known of the rest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub hits: Vec<Hit>,
    /// How many results there are in all; `None` when a date range stopped checking
    /// before it reached the last one.
    pub total: Option<usize>,
    /// True when another page follows this one.
    pub more: bool,
    /// True when a date range was checked on only the best name matches, so more
    /// results may exist for a narrower name.
    pub partial: bool,
    /// Bytes of memory the search index holds, shown beside the search time.
    pub index_bytes: usize,
}

/// `count` results from rank `offset` for `text` narrowed by `filters`. There is no
/// cap: pages can be asked for until `more` is false. `None` when a newer search
/// replaced this one before it finished.
pub fn search(
    db: &Db,
    index: &IndexService,
    text: &str,
    filters: &Filters,
    offset: usize,
    count: usize,
) -> AppResult<Option<Answer>> {
    let cancelled = index.begin_search();
    answer(
        db,
        index,
        &query_for(text, filters),
        filters,
        offset,
        count,
        &cancelled,
    )
}

/// The typed query with the filter bar's choices added.
pub fn query_for(text: &str, filters: &Filters) -> Query {
    let mut query = search::parse(text);
    query.narrow(filters);
    query
}

/// [`search`] for a query already parsed, under a search already begun: `None` once
/// `cancelled` reports that a newer search replaced it.
pub fn answer(
    db: &Db,
    index: &IndexService,
    query: &Query,
    filters: &Filters,
    offset: usize,
    count: usize,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> AppResult<Option<Answer>> {
    let index_bytes = index.memory_bytes();
    let Some(range) = filters.modified() else {
        let (hits, total) = if query.tags().is_empty() {
            match index.search(query, offset, count, cancelled) {
                Some(page) => (page.hits, page.total),
                None => return Ok(None),
            }
        } else {
            let all = tagged(db, query)?;
            let total = all.len();
            (all.into_iter().skip(offset).take(count).collect(), total)
        };
        return Ok(Some(Answer {
            more: offset.saturating_add(hits.len()) < total,
            hits,
            total: Some(total),
            partial: false,
            index_bytes,
        }));
    };
    // A date rejects most name matches, so more of them are checked than are shown.
    let wanted = offset.saturating_add(count);
    let pool = dated::pool_for(wanted).max(wanted);
    let named = if query.tags().is_empty() {
        match index.search(query, 0, pool, cancelled) {
            Some(page) => page.hits,
            None => return Ok(None),
        }
    } else {
        let mut all = tagged(db, query)?;
        all.truncate(pool);
        all
    };
    let pool_full = named.len() >= pool;
    // One more than the page asks for tells whether another page follows.
    let Some(kept) = dated::keep(named, &range, wanted.saturating_add(1), cancelled) else {
        return Ok(None);
    };
    let more = kept.len() > wanted;
    Ok(Some(Answer {
        total: (!pool_full && !more).then_some(kept.len()),
        partial: pool_full && !more,
        more,
        hits: kept.into_iter().skip(offset).take(count).collect(),
        index_bytes,
    }))
}

/// The tagged folders that still exist and match the query's name terms, best first.
fn tagged(db: &Db, query: &Query) -> AppResult<Vec<Hit>> {
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
    Ok(ranked.into_iter().map(|(_, hit)| hit).collect())
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

    fn typed(db: &Db, index: &IndexService, text: &str, count: usize) -> AppResult<Option<Answer>> {
        search(db, index, text, &Filters::default(), 0, count)
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
                total: Some(0),
                more: false,
                partial: false,
                index_bytes: 0
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
        let first = typed(&db, &index, "tag:work", 1).unwrap().unwrap();
        assert_eq!(
            (first.hits.len(), first.total, first.more),
            (1, Some(2), true)
        );
        let second = search(&db, &index, "tag:work", &Filters::default(), 1, 1)
            .unwrap()
            .unwrap();
        assert_eq!(second.hits[0].name, "proj-alpha");
        assert!(!second.more);

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
        assert_eq!(
            names(search(&db, &index, "", &by_tag, 0, 10)),
            ["flt-alpha"]
        );
        let files_only = Filters {
            kind: Kind::File,
            ..by_tag
        };
        assert!(names(search(&db, &index, "", &files_only, 0, 10)).is_empty());
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
        let answer = search(&db, &index, "report", &last_month, 0, 10)
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
        let answer = search(&db, &index, "log", &recent, 0, 10).unwrap().unwrap();
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
                0,
                10
            )),
            ["dated-tagged"]
        );
        assert!(names(search(&db, &index, "", &tagged(None, Some(1_000)), 0, 10)).is_empty());
    }

    #[test]
    fn names_come_in_pages_until_every_match_is_listed() {
        let names: Vec<String> = (0..400).map(|i| format!("log{i:03}.txt")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let index = rooted(&std::env::temp_dir(), &refs);
        let db = db::open_in_memory().unwrap();
        let page = |offset| {
            search(&db, &index, "log", &Filters::default(), offset, 150)
                .unwrap()
                .unwrap()
        };
        let first = page(0);
        assert_eq!(
            (first.hits.len(), first.total, first.more),
            (150, Some(400), true)
        );
        assert_eq!(first.hits[0].name, "log000.txt");
        let last = page(300);
        assert_eq!(
            (last.hits.len(), last.total, last.more),
            (100, Some(400), false)
        );
        assert_eq!(last.hits[99].name, "log399.txt");
    }

    #[test]
    fn a_dated_search_pages_too() {
        let dir = std::env::temp_dir().join(format!("findex-find-paged-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let now = now_ms();
        let names: Vec<String> = (0..5).map(|i| format!("note{i}.txt")).collect();
        for name in &names {
            touch(&dir.join(name), now - 1_000);
        }
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let index = rooted(&dir, &refs);
        let db = db::open_in_memory().unwrap();
        let recent = Filters {
            modified_from: Some(now - 60_000),
            ..Filters::default()
        };
        let page = |offset| {
            search(&db, &index, "note", &recent, offset, 2)
                .unwrap()
                .unwrap()
        };
        let (first, last) = (page(0), page(4));
        std::fs::remove_dir_all(&dir).unwrap();
        // The count is unknown until the dates of every match were read.
        assert_eq!((first.hits.len(), first.total, first.more), (2, None, true));
        assert_eq!(
            (last.hits.len(), last.total, last.more),
            (1, Some(5), false)
        );
        assert!(!last.partial);
    }

    #[test]
    fn every_answer_says_how_much_memory_the_index_holds() {
        let index = rooted(&std::env::temp_dir(), &["notes.txt", "report.pdf"]);
        let db = db::open_in_memory().unwrap();
        let answer = search(&db, &index, "notes", &Filters::default(), 0, 10)
            .unwrap()
            .unwrap();
        assert!(answer.index_bytes > 0);
        assert_eq!(answer.index_bytes, index.memory_bytes());
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
