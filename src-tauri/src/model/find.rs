//! Answers a search box query: names come from the in-memory index, while a query
//! with `tag:` filters is answered from the tags table, which the index knows
//! nothing about.

use std::path::Path;

use crate::error::AppResult;
use crate::model::db::Db;
use crate::model::index::service::IndexService;
use crate::model::search::{self, Hit};
use crate::model::tags;

/// The best `limit` results for `text`. `None` when a newer index search replaced
/// this one; a tag query is answered at once and never is.
pub fn search(
    db: &Db,
    index: &IndexService,
    text: &str,
    limit: usize,
) -> AppResult<Option<Vec<Hit>>> {
    let query = search::parse(text);
    if query.tags().is_empty() {
        return Ok(index.search(text, limit));
    }
    let mut ranked: Vec<(u32, Hit)> = tags::folders_with(db, query.tags())?
        .into_iter()
        .filter(|path| Path::new(path).is_dir())
        .filter_map(|path| {
            // A drive root has no name of its own; it is shown by its path.
            let name = Path::new(&path)
                .file_name()
                .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
            let rank = search::rank_name(&query, &name, true)?;
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
    Ok(Some(
        ranked.into_iter().take(limit).map(|(_, hit)| hit).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::db;

    fn folder(name: &str) -> String {
        let dir = std::env::temp_dir()
            .join(format!("findex-find-{}", std::process::id()))
            .join(name);
        std::fs::create_dir_all(&dir).unwrap();
        dir.display().to_string()
    }

    fn names(hits: Option<Vec<Hit>>) -> Vec<String> {
        hits.unwrap().into_iter().map(|hit| hit.name).collect()
    }

    #[test]
    fn a_query_without_tags_goes_to_the_index() {
        let db = db::open_in_memory().unwrap();
        tags::add(&db, &folder("proj-index"), "work").unwrap();
        let index = IndexService::default();
        assert_eq!(search(&db, &index, "proj", 10).unwrap(), Some(Vec::new()));
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
            names(search(&db, &index, "tag:work", 10).unwrap()),
            ["notes", "proj-alpha"]
        );
        assert_eq!(
            names(search(&db, &index, "proj tag:work", 10).unwrap()),
            ["proj-alpha"]
        );
        assert_eq!(
            names(search(&db, &index, "tag:work;fw", 10).unwrap()),
            Vec::<String>::new()
        );
        assert_eq!(names(search(&db, &index, "tag:work", 1).unwrap()).len(), 1);

        let hits = search(&db, &index, "tag:fw", 10).unwrap().unwrap();
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
        assert!(names(search(&db, &index, "tag:old", 10).unwrap()).is_empty());
    }
}
