//! Name search over the volume indexes.
//!
//! A query's longest term is searched with one SIMD substring pass over each volume's
//! lowercase arena, split into chunks that run in parallel. Each hit is mapped back to
//! its node, checked against the other terms and filters, counted, and offered to a
//! bounded per-chunk top-K. Paths are rebuilt only for the page asked for, so the cost
//! of a query grows with the matches, not with the size of the drive.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::ops::Range;

use memchr::memchr;
use memchr::memmem::Finder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::model::dated;
use crate::model::index::store::{VolumeIndex, SEPARATOR};

/// Bytes of arena per parallel task; large enough to amortize the task, small enough
/// to keep every core busy on one drive.
const CHUNK_BYTES: usize = 1 << 20;

/// Nodes per parallel task for a filter-only query, which has no term to search for.
const CHUNK_NODES: usize = 65_536;

/// How many matches a task handles between two cancellation checks.
const CANCEL_EVERY: u32 = 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Any,
    File,
    Folder,
}

/// What the filter bar adds to the typed query. Every field narrows the results;
/// the default narrows nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Filters {
    pub kind: Kind,
    pub tags: Vec<String>,
    /// File types to keep, by extension (`c`, `.h`); any of them matches.
    pub exts: Vec<String>,
    /// Last changed at or after, in milliseconds since the Unix epoch.
    pub modified_from: Option<i64>,
    /// Last changed before, in milliseconds since the Unix epoch.
    pub modified_to: Option<i64>,
}

impl Filters {
    /// The last-change range to keep, when either bound is set.
    pub fn modified(&self) -> Option<dated::Range> {
        dated::Range::new(self.modified_from, self.modified_to)
    }
}

/// A parsed query: every term must appear in the name; filters narrow further.
#[derive(Debug)]
pub struct Query {
    terms: Vec<String>,
    exts: Vec<String>,
    kind: Kind,
    /// `tag:` filters; the index holds no tags, so `find` answers these from the tags table.
    tags: Vec<String>,
}

impl Query {
    /// True when nothing in the query selects anything; such a query lists nothing.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty() && self.exts.is_empty()
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// The words every result must hold, lowercased.
    pub fn terms(&self) -> &[String] {
        &self.terms
    }

    /// The file types kept, without the dot; empty keeps every type.
    pub fn exts(&self) -> &[String] {
        &self.exts
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Adds the filter bar's choices: its kind replaces a typed `type:`, its tags join
    /// the typed `tag:` ones, and its file types join the typed `ext:` ones.
    pub fn narrow(&mut self, filters: &Filters) {
        if filters.kind != Kind::Any {
            self.kind = filters.kind;
        }
        self.exts.extend(
            filters
                .exts
                .iter()
                .map(|ext| ext.trim().trim_start_matches('.').to_lowercase())
                .filter(|ext| !ext.is_empty()),
        );
        self.tags.extend(
            filters
                .tags
                .iter()
                .map(|tag| tag.trim().trim_start_matches('#').to_lowercase())
                .filter(|tag| !tag.is_empty()),
        );
    }
}

/// One result row: no file metadata, so no disk access per result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

/// Reads `report ext:pdf;docx type:file` style input. Terms are lowercased the same
/// way names are, so matching ignores case in any script.
pub fn parse(text: &str) -> Query {
    let mut query = Query {
        terms: Vec::new(),
        exts: Vec::new(),
        kind: Kind::Any,
        tags: Vec::new(),
    };
    for token in text.split_whitespace() {
        let lower = token.to_lowercase();
        if let Some(list) = lower.strip_prefix("ext:") {
            query.exts.extend(
                list.split([';', ','])
                    .map(|ext| ext.trim_start_matches('.'))
                    .filter(|ext| !ext.is_empty())
                    .map(str::to_owned),
            );
        } else if let Some(list) = lower.strip_prefix("tag:") {
            query.tags.extend(
                list.split([';', ','])
                    .map(|tag| tag.trim_start_matches('#'))
                    .filter(|tag| !tag.is_empty())
                    .map(str::to_owned),
            );
        } else if let Some(kind) = lower.strip_prefix("type:").and_then(kind_of) {
            query.kind = kind;
        } else {
            query.terms.push(lower);
        }
    }
    query
}

/// How well one name answers `query`, ranked as the index search ranks it; `None`
/// when it does not match. For results found outside the index, such as tags.
pub fn rank_name(query: &Query, name: &str, is_dir: bool) -> Option<u32> {
    let lower = name.to_lowercase();
    // The matcher leaves the longest term to the arena search; here it is checked too.
    if !query.terms.iter().all(|term| lower.contains(term.as_str())) {
        return None;
    }
    Matcher::new(query).check(&lower, is_dir)
}

fn kind_of(word: &str) -> Option<Kind> {
    match word {
        "file" | "files" => Some(Kind::File),
        "folder" | "folders" | "dir" | "dirs" => Some(Kind::Folder),
        _ => None,
    }
}

/// One page of ranked results, and how many names matched in all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub hits: Vec<Hit>,
    pub total: usize,
}

/// Runs `query` over every volume and returns `count` hits starting at rank `offset`,
/// best first, with the number of every match. Each page ranks the whole drive again,
/// so the pages of one query follow one ranking. Returns `None` as soon as `cancelled`
/// reports that a newer query replaced this one.
pub fn run(
    volumes: &[VolumeIndex],
    query: &Query,
    offset: usize,
    count: usize,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Option<Page> {
    if query.is_empty() {
        return Some(Page {
            hits: Vec::new(),
            total: 0,
        });
    }
    let limit = offset.saturating_add(count);
    let matcher = Matcher::new(query);
    let mut best = Vec::new();
    let mut total = 0;
    for (vol, index) in volumes.iter().enumerate() {
        let parts: Vec<Option<Part>> = match &matcher.primary {
            Some(primary) => chunks(index.lower_arena(), CHUNK_BYTES)
                .into_par_iter()
                .map(|range| scan_arena(vol, index, &matcher, primary, range, limit, cancelled))
                .collect(),
            None => (0..index.node_count())
                .step_by(CHUNK_NODES)
                .collect::<Vec<_>>()
                .into_par_iter()
                .map(|start| {
                    let end = start
                        .saturating_add(CHUNK_NODES as u32)
                        .min(index.node_count());
                    scan_nodes(vol, index, &matcher, start..end, limit, cancelled)
                })
                .collect(),
        };
        for part in parts {
            let part = part?;
            total += part.matched;
            best.extend(part.top);
        }
    }
    best.sort_unstable();
    best.truncate(limit);
    let hits = best
        .into_iter()
        .skip(offset)
        .map(|c| {
            let index = &volumes[c.vol];
            Hit {
                name: index.name(c.id).to_owned(),
                path: index.path(c.id),
                is_dir: c.is_dir,
            }
        })
        .collect();
    Some(Page { hits, total })
}

/// What one parallel task found: its best candidates and how many names matched.
struct Part<'a> {
    top: Vec<Candidate<'a>>,
    matched: usize,
}

/// The compiled form of a query: the term searched across the arena, and the checks
/// a candidate must pass after it is found.
struct Matcher<'q> {
    primary: Option<Finder<'q>>,
    others: Vec<Finder<'q>>,
    terms: &'q [String],
    exts: &'q [String],
    kind: Kind,
}

impl<'q> Matcher<'q> {
    fn new(query: &'q Query) -> Self {
        // The longest term is the most selective guess, so it drives the arena search.
        let longest = (0..query.terms.len()).max_by_key(|&i| query.terms[i].len());
        let finder = |i: usize| Finder::new(query.terms[i].as_bytes());
        Self {
            primary: longest.map(finder),
            others: (0..query.terms.len())
                .filter(|&i| Some(i) != longest)
                .map(finder)
                .collect(),
            terms: &query.terms,
            exts: &query.exts,
            kind: query.kind,
        }
    }

    /// The rank tier of a name that passes every check, or `None`.
    fn check(&self, lower: &str, is_dir: bool) -> Option<u32> {
        match self.kind {
            Kind::File if is_dir => return None,
            Kind::Folder if !is_dir => return None,
            _ => {}
        }
        if !self.exts.is_empty() {
            let ext = lower.rsplit_once('.')?.1;
            if !self.exts.iter().any(|e| e == ext) {
                return None;
            }
        }
        if !self
            .others
            .iter()
            .all(|f| f.find(lower.as_bytes()).is_some())
        {
            return None;
        }
        Some(self.terms.iter().map(|term| tier(lower, term)).sum())
    }
}

/// How well one term matches a name: 3 for the whole name or the name without its
/// extension, 2 for a prefix, 1 for the start of a word, 0 for anywhere else.
fn tier(name: &str, term: &str) -> u32 {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    if name == term || stem == term {
        3
    } else if name.starts_with(term) {
        2
    } else if name.match_indices(term).any(|(i, _)| {
        name[..i]
            .chars()
            .next_back()
            .is_some_and(|c| !c.is_alphanumeric())
    }) {
        1
    } else {
        0
    }
}

fn scan_arena<'a>(
    vol: usize,
    index: &'a VolumeIndex,
    matcher: &Matcher<'_>,
    primary: &Finder<'_>,
    range: Range<usize>,
    limit: usize,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Option<Part<'a>> {
    if cancelled() {
        return None;
    }
    let hay = index.lower_arena();
    let mut top = TopK::new(limit);
    let mut matched = 0;
    let mut pos = range.start;
    let mut seen = 0_u32;
    while let Some(found) = primary.find(&hay[pos..range.end]) {
        let at = pos + found;
        if let Some(id) = index.owner_of(at) {
            let lower = index.lower_name(id);
            let is_dir = index.is_dir(id);
            if let Some(tier) = matcher.check(lower, is_dir) {
                matched += 1;
                top.offer(Candidate::new(tier, lower, is_dir, vol, id));
            }
        }
        // One hit per name: resume after this name's separator.
        pos = memchr(SEPARATOR, &hay[at..range.end]).map_or(range.end, |end| at + end + 1);
        seen += 1;
        if seen % CANCEL_EVERY == 0 && cancelled() {
            return None;
        }
    }
    Some(Part {
        top: top.into_vec(),
        matched,
    })
}

fn scan_nodes<'a>(
    vol: usize,
    index: &'a VolumeIndex,
    matcher: &Matcher<'_>,
    ids: Range<u32>,
    limit: usize,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Option<Part<'a>> {
    if cancelled() {
        return None;
    }
    let mut top = TopK::new(limit);
    let mut matched = 0;
    for id in ids.filter(|&id| index.is_live(id)) {
        let lower = index.lower_name(id);
        let is_dir = index.is_dir(id);
        if let Some(tier) = matcher.check(lower, is_dir) {
            matched += 1;
            top.offer(Candidate::new(tier, lower, is_dir, vol, id));
        }
    }
    Some(Part {
        top: top.into_vec(),
        matched,
    })
}

/// Splits the arena into ranges of about `size` bytes that end on a separator, so no
/// name is cut between two tasks.
fn chunks(hay: &[u8], size: usize) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < hay.len() {
        let target = (start + size).min(hay.len());
        let end = memchr(SEPARATOR, &hay[target..]).map_or(hay.len(), |at| target + at + 1);
        out.push(start..end);
        start = end;
    }
    out
}

/// A match waiting to be ranked. Ordering puts the better match first: higher tier,
/// then the shorter name, then folders, then alphabetical, so ties are stable.
struct Candidate<'a> {
    tier: u32,
    len: usize,
    is_dir: bool,
    lower: &'a str,
    vol: usize,
    id: u32,
}

impl<'a> Candidate<'a> {
    fn new(tier: u32, lower: &'a str, is_dir: bool, vol: usize, id: u32) -> Self {
        Self {
            tier,
            len: lower.chars().count(),
            is_dir,
            lower,
            vol,
            id,
        }
    }
}

impl Ord for Candidate<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .tier
            .cmp(&self.tier)
            .then(self.len.cmp(&other.len))
            .then(other.is_dir.cmp(&self.is_dir))
            .then(self.lower.cmp(other.lower))
            .then(self.vol.cmp(&other.vol))
            .then(self.id.cmp(&other.id))
    }
}

impl PartialOrd for Candidate<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Candidate<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Candidate<'_> {}

/// Keeps the best `limit` candidates; the heap's top is the worst one kept.
struct TopK<'a> {
    heap: BinaryHeap<Candidate<'a>>,
    limit: usize,
}

impl<'a> TopK<'a> {
    fn new(limit: usize) -> Self {
        Self {
            heap: BinaryHeap::with_capacity(limit.min(4096) + 1),
            limit,
        }
    }

    fn offer(&mut self, candidate: Candidate<'a>) {
        if self.heap.len() < self.limit {
            self.heap.push(candidate);
        } else if self.heap.peek().is_some_and(|worst| candidate < *worst) {
            self.heap.pop();
            self.heap.push(candidate);
        }
    }

    fn into_vec(self) -> Vec<Candidate<'a>> {
        self.heap.into_vec()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering as Atomic};
    use std::time::Instant;

    use super::*;

    const ROOT: u64 = 5;

    fn index(entries: &[(u64, u64, &str, bool)]) -> VolumeIndex {
        let mut index = VolumeIndex::new("C:\\", ROOT);
        for &(frn, parent, name, is_dir) in entries {
            index.upsert(frn, parent, name, is_dir).unwrap();
        }
        index
    }

    const PAGE: usize = 200;

    fn names(volumes: &[VolumeIndex], text: &str) -> Vec<String> {
        run(volumes, &parse(text), 0, PAGE, &|| false)
            .unwrap()
            .hits
            .into_iter()
            .map(|hit| hit.name)
            .collect()
    }

    #[test]
    fn parses_terms_extensions_and_type() {
        let query = parse("  Annual REPORT ext:.PDF;docx type:file ");
        assert_eq!(query.terms, ["annual", "report"]);
        assert_eq!(query.exts, ["pdf", "docx"]);
        assert_eq!(query.kind, Kind::File);
        // An unknown type stays a search term rather than vanishing.
        assert_eq!(parse("type:song").terms, ["type:song"]);
    }

    #[test]
    fn every_term_must_appear_in_the_name() {
        let volumes = [index(&[
            (10, ROOT, "annual report 2025.pdf", false),
            (11, ROOT, "annual budget.xlsx", false),
            (12, ROOT, "report.txt", false),
        ])];
        assert_eq!(names(&volumes, "report annual"), ["annual report 2025.pdf"]);
    }

    #[test]
    fn filters_by_extension_and_type() {
        let volumes = [index(&[
            (10, ROOT, "notes.pdf", false),
            (11, ROOT, "notes.md", false),
            (12, ROOT, "notes", true),
        ])];
        assert_eq!(names(&volumes, "notes ext:pdf"), ["notes.pdf"]);
        assert_eq!(names(&volumes, "notes type:folder"), ["notes"]);
        assert_eq!(names(&volumes, "ext:md"), ["notes.md"]);
    }

    #[test]
    fn tag_filters_are_kept_apart_from_name_terms() {
        let query = parse("proj tag:Work;fw TAG:esp");
        assert_eq!(query.tags(), ["work", "fw", "esp"]);
        assert_eq!(query.terms, ["proj"]);
    }

    #[test]
    fn the_filter_bar_narrows_a_typed_query() {
        let mut query = parse("proj tag:work type:file");
        query.narrow(&Filters {
            kind: Kind::Folder,
            tags: vec!["#FW".into(), " ".into()],
            ..Filters::default()
        });
        assert_eq!(query.kind, Kind::Folder);
        assert_eq!(query.tags(), ["work", "fw"]);
        assert_eq!(query.terms, ["proj"]);

        // "Any" leaves a typed type: alone.
        let mut typed = parse("notes type:file");
        typed.narrow(&Filters::default());
        assert_eq!(typed.kind, Kind::File);
    }

    #[test]
    fn the_type_filter_keeps_only_its_extensions() {
        let volumes = [index(&[
            (10, ROOT, "code.c", false),
            (11, ROOT, "code.H", false),
            (12, ROOT, "code.txt", false),
            (13, ROOT, "code.pdf", false),
            (14, ROOT, "code", true),
        ])];
        let by_type = |text: &str, exts: &[&str]| {
            let mut query = parse(text);
            query.narrow(&Filters {
                exts: exts.iter().map(|e| (*e).to_owned()).collect(),
                ..Filters::default()
            });
            let mut names: Vec<String> = run(&volumes, &query, 0, PAGE, &|| false)
                .unwrap()
                .hits
                .into_iter()
                .map(|hit| hit.name)
                .collect();
            names.sort();
            names
        };
        assert_eq!(by_type("code", &["c", ".H", " "]), ["code.H", "code.c"]);
        // A typed ext: and the filter's types are both accepted.
        assert_eq!(by_type("code ext:pdf", &["c"]), ["code.c", "code.pdf"]);
        // The filter alone lists every name of that type.
        assert_eq!(by_type("", &["txt"]), ["code.txt"]);
    }

    #[test]
    fn filters_arrive_from_the_frontend_in_camel_case() {
        let filters: Filters = serde_json::from_str(
            r#"{"kind":"file","tags":["work"],"exts":["c"],"modifiedFrom":10,"modifiedTo":null}"#,
        )
        .unwrap();
        assert_eq!(filters.kind, Kind::File);
        assert_eq!(filters.tags, ["work"]);
        assert_eq!(filters.exts, ["c"]);
        assert_eq!(filters.modified(), dated::Range::new(Some(10), None));
        let empty: Filters = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, Filters::default());
        assert_eq!(empty.modified(), None);
    }

    #[test]
    fn ranks_one_name_like_the_index_search_does() {
        let query = parse("report ext:pdf");
        assert_eq!(rank_name(&query, "Report.PDF", false), Some(3));
        assert_eq!(rank_name(&query, "annual-report.pdf", false), Some(1));
        assert_eq!(rank_name(&query, "report.txt", false), None);
        assert_eq!(rank_name(&parse("tag:x"), "Anything", true), Some(0));
        assert_eq!(rank_name(&parse("type:file tag:x"), "Folder", true), None);
    }

    #[test]
    fn ranks_exact_then_prefix_then_word_then_substring() {
        let volumes = [index(&[
            (10, ROOT, "myreport.txt", false),
            (11, ROOT, "annual-report.txt", false),
            (12, ROOT, "reports.txt", false),
            (13, ROOT, "report.txt", false),
            (14, ROOT, "Report", true),
        ])];
        assert_eq!(
            names(&volumes, "report"),
            [
                "Report",
                "report.txt",
                "reports.txt",
                "annual-report.txt",
                "myreport.txt"
            ]
        );
    }

    #[test]
    fn matching_ignores_case_in_any_script() {
        let volumes = [index(&[
            (10, ROOT, "Ảnh Cưới.JPG", false),
            (11, ROOT, "anh.txt", false),
        ])];
        assert_eq!(names(&volumes, "ẢNH"), ["Ảnh Cưới.JPG"]);
        assert_eq!(names(&volumes, "cưới ext:jpg"), ["Ảnh Cưới.JPG"]);
    }

    #[test]
    fn returns_full_paths_across_volumes() {
        let c = index(&[(10, ROOT, "Users", true), (11, 10, "plan.md", false)]);
        let mut d = VolumeIndex::new("D:\\", ROOT);
        d.upsert(20, ROOT, "plan.md", false).unwrap();
        let hits = run(&[c, d], &parse("plan"), 0, PAGE, &|| false)
            .unwrap()
            .hits;
        let paths: Vec<_> = hits.iter().map(|hit| hit.path.as_str()).collect();
        assert_eq!(paths, ["C:\\Users\\plan.md", "D:\\plan.md"]);
    }

    #[test]
    fn old_and_deleted_names_never_match() {
        let mut volume = index(&[(10, ROOT, "draft.txt", false), (11, ROOT, "drafts", true)]);
        volume.upsert(10, ROOT, "final.txt", false).unwrap();
        volume.remove(11);
        assert!(names(&[volume], "draft").is_empty());
    }

    fn logs(count: u64) -> VolumeIndex {
        let mut volume = VolumeIndex::new("C:\\", ROOT);
        for i in 0..count {
            volume
                .upsert(100 + i, ROOT, &format!("log{i:03}.txt"), false)
                .unwrap();
        }
        volume
    }

    #[test]
    fn keeps_only_the_best_page_and_counts_every_match() {
        let page = run(&[logs(500)], &parse("log"), 0, 3, &|| false).unwrap();
        let got: Vec<_> = page.hits.iter().map(|hit| hit.name.as_str()).collect();
        assert_eq!(got, ["log000.txt", "log001.txt", "log002.txt"]);
        assert_eq!(page.total, 500);
    }

    #[test]
    fn pages_follow_one_ranking_until_the_matches_run_out() {
        let volumes = [logs(500)];
        let query = parse("log");
        let whole = run(&volumes, &query, 0, 500, &|| false).unwrap().hits;
        let mut paged = Vec::new();
        for offset in [0, 200, 400] {
            let page = run(&volumes, &query, offset, 200, &|| false).unwrap();
            assert_eq!(page.total, 500);
            paged.extend(page.hits);
        }
        assert_eq!(paged.len(), 500);
        assert_eq!(paged, whole);
        // Past the end there is nothing more, and the count still holds.
        let after = run(&volumes, &query, 500, 200, &|| false).unwrap();
        assert!(after.hits.is_empty());
        assert_eq!(after.total, 500);
        // A filter-only query counts too.
        assert_eq!(
            run(&volumes, &parse("ext:txt"), 0, 10, &|| false)
                .unwrap()
                .total,
            500
        );
    }

    #[test]
    fn an_empty_query_lists_nothing() {
        let volumes = [index(&[(10, ROOT, "a.txt", false)])];
        assert!(names(&volumes, "   ").is_empty());
        assert!(names(&volumes, "type:file").is_empty());
    }

    #[test]
    fn a_cancelled_query_returns_none() {
        let volumes = [index(&[(10, ROOT, "a.txt", false)])];
        let stop = AtomicBool::new(true);
        assert!(run(&volumes, &parse("a"), 0, 10, &|| stop.load(Atomic::Relaxed)).is_none());
    }

    #[test]
    fn chunks_end_on_separators() {
        let hay = b"alpha\0beta\0gamma\0";
        let ranges = chunks(hay, 3);
        assert_eq!(ranges, [0..6, 6..11, 11..17]);
    }

    /// Release-mode benchmark over one million synthetic names:
    /// `cargo test --release --lib -- --ignored perf_one_million --nocapture`
    /// (`--lib`: the release app binary asks for administrator rights)
    #[test]
    #[ignore]
    fn perf_one_million() {
        const WORDS: [&str; 24] = [
            "report",
            "invoice",
            "photo",
            "backup",
            "project",
            "notes",
            "draft",
            "final",
            "budget",
            "meeting",
            "design",
            "schematic",
            "firmware",
            "driver",
            "config",
            "readme",
            "build",
            "test",
            "data",
            "image",
            "video",
            "music",
            "archive",
            "log",
        ];
        const EXTS: [&str; 8] = ["pdf", "txt", "c", "h", "rs", "png", "docx", "json"];
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut volume = VolumeIndex::new("C:\\", ROOT);
        let mut dirs = vec![ROOT];
        for frn in 100..1_100_000_u64 {
            let r = next();
            let parent = dirs[(r % dirs.len() as u64) as usize];
            let a = WORDS[(r >> 8) as usize % WORDS.len()];
            let b = WORDS[(r >> 16) as usize % WORDS.len()];
            let is_dir = r % 10 == 0;
            let name = if is_dir {
                format!("{a} {b}")
            } else {
                format!(
                    "{a}_{b}_{}.{}",
                    (r >> 24) % 10_000,
                    EXTS[(r >> 40) as usize % EXTS.len()]
                )
            };
            volume.upsert(frn, parent, &name, is_dir).unwrap();
            if is_dir {
                dirs.push(frn);
            }
        }
        let volumes = [volume];
        let mut worst = 0.0_f64;
        for text in [
            "report",
            "a",
            "final budget",
            "ext:pdf",
            "zzzz",
            "schematic ext:png",
            "e",
        ] {
            let query = parse(text);
            let mut times: Vec<f64> = (0..20)
                .map(|_| {
                    let start = Instant::now();
                    let hits = run(&volumes, &query, 0, PAGE, &|| false).unwrap();
                    std::hint::black_box(hits);
                    start.elapsed().as_secs_f64() * 1000.0
                })
                .collect();
            times.sort_by(f64::total_cmp);
            let (p50, p95) = (times[10], times[18]);
            println!("{text:>20}: p50 {p50:6.2} ms  p95 {p95:6.2} ms");
            worst = worst.max(p95);
        }
        assert!(worst <= 50.0, "p95 {worst:.2} ms is over the 50 ms budget");
    }
}
