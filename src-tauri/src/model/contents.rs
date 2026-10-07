//! Searches inside files. Nothing is indexed: the files of the active drive whose type
//! holds text are read on demand, in parallel, and each match is handed on as soon as
//! it is found, so the first results show while the rest of the drive is still being
//! read. Files whose name matches come first, ranked, since they need no disk. A newer
//! search stops the scan between two files.

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use memchr::{memchr, memchr_iter, memrchr};
use rayon::prelude::*;
use regex::bytes::{Regex, RegexBuilder};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::model::dated;
use crate::model::db::Db;
use crate::model::details;
use crate::model::find;
use crate::model::index::service::IndexService;
use crate::model::search::{Filters, Hit, Kind, Query};
use crate::model::tags;

/// The largest file read; bigger ones are mostly data, not text someone wrote.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Bytes looked at for a NUL, which marks a binary file.
const SNIFF_BYTES: usize = 8 * 1024;

/// Content matches sent together; a batch also goes out every `FLUSH_EVERY`, so the
/// count of files read keeps moving while nothing matches.
const BATCH_HITS: usize = 64;
const FLUSH_EVERY: Duration = Duration::from_millis(150);

/// Name matches sent per batch, before the contents are read.
const NAME_BATCH: usize = 500;

/// Characters of a matching line shown: up to `LEAD_CHARS` before the match, the
/// rest after it.
const SNIPPET_CHARS: usize = 160;
const LEAD_CHARS: usize = 40;

/// Threads reading files: enough to keep a disk busy, few enough to leave the rest of
/// the app responsive.
const MAX_THREADS: usize = 8;

/// File types read when no type is picked: code, text, markup, configuration, logs,
/// and design files saved as text (KiCad).
const TEXT_EXTS: &[&str] = &[
    "c",
    "h",
    "cpp",
    "hpp",
    "cc",
    "cxx",
    "hh",
    "hxx",
    "ino",
    "s",
    "asm",
    "inc",
    "ld",
    "mk",
    "cmake",
    "rs",
    "py",
    "pyi",
    "js",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "jsx",
    "java",
    "kt",
    "kts",
    "go",
    "cs",
    "swift",
    "m",
    "mm",
    "rb",
    "php",
    "pl",
    "lua",
    "sh",
    "bash",
    "zsh",
    "ps1",
    "psm1",
    "bat",
    "cmd",
    "sql",
    "r",
    "dart",
    "scala",
    "vue",
    "svelte",
    "html",
    "htm",
    "css",
    "scss",
    "sass",
    "less",
    "xml",
    "xsd",
    "svg",
    "json",
    "jsonc",
    "yaml",
    "yml",
    "toml",
    "ini",
    "cfg",
    "conf",
    "config",
    "properties",
    "env",
    "txt",
    "md",
    "markdown",
    "rst",
    "adoc",
    "tex",
    "csv",
    "tsv",
    "log",
    "gradle",
    "proto",
    "dts",
    "dtsi",
    "overlay",
    "kicad_sch",
    "kicad_pcb",
    "kicad_pro",
    "kicad_sym",
    "kicad_mod",
    "kicad_dru",
    "gitignore",
    "editorconfig",
];

/// Files without a type that hold text.
const TEXT_NAMES: &[&str] = &[
    "makefile",
    "kconfig",
    "dockerfile",
    "jenkinsfile",
    "readme",
    "license",
];

/// One result: a file found by its name, or by a line inside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    #[serde(flatten)]
    pub hit: Hit,
    /// The line of the first match, counted from 1; absent for a match on the name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// The text of that line around the match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

/// What a scan sends as it goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    pub hits: Vec<Found>,
    /// Files looked at so far, read or skipped (too large, binary, locked, or
    /// outside the date range).
    pub files_read: usize,
    /// Files to look at in all; 0 while the name matches are being sent.
    pub files_total: usize,
    /// The last batch of a scan that ran to its end.
    pub done: bool,
}

/// Searches names and contents for `text` narrowed by `filters`, sending what it
/// finds to `emit` as it goes. A newer search stops it, and then no batch follows.
pub fn scan(
    db: &Db,
    index: &IndexService,
    text: &str,
    filters: &Filters,
    emit: &(dyn Fn(Batch) + Sync),
) -> AppResult<()> {
    let cancelled = index.begin_search();
    scan_with(
        db,
        index,
        &find::query_for(text, filters),
        filters,
        &cancelled,
        emit,
    )
}

fn scan_with(
    db: &Db,
    index: &IndexService,
    query: &Query,
    filters: &Filters,
    cancelled: &(dyn Fn() -> bool + Sync),
    emit: &(dyn Fn(Batch) + Sync),
) -> AppResult<()> {
    // Names first: they need no disk, and they come ranked.
    let Some(named) = find::answer(db, index, query, filters, 0, usize::MAX, cancelled)? else {
        return Ok(());
    };
    for chunk in named.hits.chunks(NAME_BATCH) {
        if cancelled() {
            return Ok(());
        }
        emit(Batch {
            hits: chunk.iter().cloned().map(by_name).collect(),
            files_read: 0,
            files_total: 0,
            done: false,
        });
    }
    let finder = Finder::new(query.terms())?;
    let paths = match finder {
        // Without a word there is nothing to look for inside; folders have no contents.
        Some(_) if query.kind() != Kind::Folder => {
            let named: HashSet<String> = named.hits.into_iter().map(|hit| hit.path).collect();
            let within = tagged_folders(db, query)?;
            let exts = query.exts();
            let Some(paths) = index.files_where(|lower| holds_text(lower, exts), cancelled) else {
                return Ok(());
            };
            paths
                .into_iter()
                .filter(|path| !named.contains(path))
                .filter(|path| within.as_ref().map_or(true, |w| is_under(path, w)))
                .collect()
        }
        _ => Vec::new(),
    };
    match finder {
        Some(finder) if !paths.is_empty() => {
            read_all(&paths, &finder, filters.modified(), cancelled, emit)
        }
        _ => {
            if !cancelled() {
                emit(Batch {
                    hits: Vec::new(),
                    files_read: 0,
                    files_total: 0,
                    done: true,
                });
            }
            Ok(())
        }
    }
}

fn by_name(hit: Hit) -> Found {
    Found {
        hit,
        line: None,
        snippet: None,
    }
}

/// Reads every path on a pool of its own and sends the matches in batches. The last
/// batch says the scan is done, unless a newer search stopped it first.
fn read_all(
    paths: &[String],
    finder: &Finder,
    range: Option<dated::Range>,
    cancelled: &(dyn Fn() -> bool + Sync),
    emit: &(dyn Fn(Batch) + Sync),
) -> AppResult<()> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(MAX_THREADS));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|err| AppError::Invalid(format!("the content search could not start: {err}")))?;
    let read = AtomicUsize::new(0);
    // The matches not sent yet, and when a batch last went out.
    let pending = Mutex::new((Vec::new(), Instant::now()));
    let total = paths.len();
    pool.install(|| {
        paths.par_iter().for_each(|path| {
            if cancelled() {
                return;
            }
            let found = look_in(path, finder, range.as_ref()).flatten();
            read.fetch_add(1, Ordering::Relaxed);
            let mut pending = pending.lock().unwrap_or_else(PoisonError::into_inner);
            pending.0.extend(found);
            if (pending.0.len() >= BATCH_HITS || pending.1.elapsed() >= FLUSH_EVERY) && !cancelled()
            {
                pending.1 = Instant::now();
                emit(Batch {
                    hits: std::mem::take(&mut pending.0),
                    files_read: read.load(Ordering::Relaxed),
                    files_total: total,
                    done: false,
                });
            }
        });
    });
    if cancelled() {
        return Ok(());
    }
    let (hits, _) = pending.into_inner().unwrap_or_else(PoisonError::into_inner);
    emit(Batch {
        hits,
        files_read: read.into_inner(),
        files_total: total,
        done: true,
    });
    Ok(())
}

/// The match in one file, if any; `None` when the file was not read: gone, locked,
/// too large, outside the date range, or binary.
fn look_in(path: &str, finder: &Finder, range: Option<&dated::Range>) -> Option<Option<Found>> {
    let file = File::open(path).ok()?;
    let meta = file.metadata().ok()?;
    if meta.len() > MAX_BYTES {
        return None;
    }
    if let Some(range) = range {
        if !details::modified_ms(&meta).is_some_and(|ms| range.contains(ms)) {
            return None;
        }
    }
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    // A file still growing is cut at the cap rather than read without end.
    file.take(MAX_BYTES).read_to_end(&mut bytes).ok()?;
    if memchr(0, &bytes[..bytes.len().min(SNIFF_BYTES)]).is_some() {
        return None;
    }
    Some(finder.find(&bytes).map(|(line, snippet)| {
        Found {
            hit: Hit {
                name: path
                    .rsplit_once('\\')
                    .map_or(path, |(_, name)| name)
                    .to_owned(),
                path: path.to_owned(),
                is_dir: false,
            },
            line: Some(line),
            snippet: Some(snippet),
        }
    }))
}

/// Whether a file, by its lowercase name, is one to read: of a picked type, or when
/// none is picked, of a type that holds text.
fn holds_text(lower: &str, exts: &[String]) -> bool {
    match lower.rsplit_once('.') {
        Some((_, ext)) if !exts.is_empty() => exts.iter().any(|e| e == ext),
        Some((_, ext)) => TEXT_EXTS.contains(&ext),
        None => exts.is_empty() && TEXT_NAMES.contains(&lower),
    }
}

/// The folders, lowercase and ending in `\`, whose files a tag filter keeps; `None`
/// when no tag is picked.
fn tagged_folders(db: &Db, query: &Query) -> AppResult<Option<Vec<String>>> {
    if query.tags().is_empty() {
        return Ok(None);
    }
    let folders = tags::folders_with(db, query.tags())?
        .into_iter()
        .map(|folder| {
            let mut folder = folder.to_lowercase();
            if !folder.ends_with('\\') {
                folder.push('\\');
            }
            folder
        })
        .collect();
    Ok(Some(folders))
}

fn is_under(path: &str, folders: &[String]) -> bool {
    let lower = path.to_lowercase();
    folders
        .iter()
        .any(|folder| lower.starts_with(folder.as_str()))
}

/// The words of a query, each matched without case in any script.
struct Finder {
    terms: Vec<Regex>,
}

impl Finder {
    /// `None` when there is no word to look for.
    fn new(terms: &[String]) -> AppResult<Option<Self>> {
        if terms.is_empty() {
            return Ok(None);
        }
        let terms = terms
            .iter()
            .map(|term| {
                RegexBuilder::new(&regex::escape(term))
                    .case_insensitive(true)
                    .build()
            })
            .collect::<Result<_, _>>()
            .map_err(|err| {
                AppError::Invalid(format!("the search words could not be read: {err}"))
            })?;
        Ok(Some(Self { terms }))
    }

    /// The line and the text around the first word's first match, when every word
    /// occurs somewhere in `bytes`.
    fn find(&self, bytes: &[u8]) -> Option<(usize, String)> {
        let mut first = None;
        for term in &self.terms {
            let found = term.find(bytes)?;
            first.get_or_insert(found.range());
        }
        let at = first?;
        Some((
            memchr_iter(b'\n', &bytes[..at.start]).count() + 1,
            snippet(bytes, at),
        ))
    }
}

/// The line holding `at`, trimmed, cut to `SNIPPET_CHARS` around the match; `…`
/// marks a cut.
fn snippet(bytes: &[u8], at: Range<usize>) -> String {
    let start = memrchr(b'\n', &bytes[..at.start]).map_or(0, |i| i + 1);
    let end = memchr(b'\n', &bytes[at.start..]).map_or(bytes.len(), |i| at.start + i);
    let before = String::from_utf8_lossy(&bytes[start..at.start]);
    let after = String::from_utf8_lossy(&bytes[at.start..end]);
    let lead: Vec<char> = before.trim_start().chars().collect();
    let mut text = if lead.len() > LEAD_CHARS {
        let kept: String = lead[lead.len() - LEAD_CHARS..].iter().collect();
        format!("…{kept}")
    } else {
        lead.iter().collect()
    };
    let after = after.trim_end();
    let room = SNIPPET_CHARS.saturating_sub(text.chars().count());
    text.extend(after.chars().take(room));
    if after.chars().count() > room {
        text.push('…');
    }
    text.replace('\t', " ")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicBool;

    use super::*;
    use crate::model::db;
    use crate::model::index::store::VolumeIndex;

    /// A folder of its own per test, removed when the test ends.
    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("findex-contents-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, name: &str, bytes: &[u8]) {
            let path = self.0.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// An index of `dir` holding `entries` as (name, parent, is_dir) below its root
    /// (frn 0); each entry's frn is its position plus one.
    fn index_of(dir: &Path, entries: &[(&str, u64, bool)]) -> IndexService {
        let mut volume = VolumeIndex::new(format!("{}\\", dir.display()), 0);
        for (i, (name, parent, is_dir)) in entries.iter().enumerate() {
            volume.upsert(i as u64 + 1, *parent, name, *is_dir).unwrap();
        }
        IndexService::with_volumes(vec![volume])
    }

    fn flat(dir: &Path, names: &[&str]) -> IndexService {
        let entries: Vec<_> = names.iter().map(|name| (*name, 0, false)).collect();
        index_of(dir, &entries)
    }

    fn run(db: &Db, index: &IndexService, text: &str, filters: &Filters) -> Vec<Batch> {
        let batches = Mutex::new(Vec::new());
        scan(db, index, text, filters, &|batch| {
            batches.lock().unwrap().push(batch)
        })
        .unwrap();
        batches.into_inner().unwrap()
    }

    /// Each result as (name, line).
    fn found(batches: &[Batch]) -> Vec<(String, Option<usize>)> {
        batches
            .iter()
            .flat_map(|batch| &batch.hits)
            .map(|found| (found.hit.name.clone(), found.line))
            .collect()
    }

    fn sorted(mut items: Vec<(String, Option<usize>)>) -> Vec<(String, Option<usize>)> {
        items.sort();
        items
    }

    #[test]
    fn finds_a_word_inside_a_file_with_its_line() {
        let temp = Temp::new("line");
        temp.write("uart.c", b"int x;\n    uart_init(9600);   \r\nreturn 0;\n");
        temp.write("notes.txt", b"nothing here\n");
        let index = flat(&temp.0, &["uart.c", "notes.txt"]);
        let db = db::open_in_memory().unwrap();
        let batches = run(&db, &index, "UART_INIT", &Filters::default());
        assert_eq!(found(&batches), [("uart.c".into(), Some(2))]);
        let hit = &batches.iter().flat_map(|b| &b.hits).next().unwrap();
        assert_eq!(hit.snippet.as_deref(), Some("uart_init(9600);"));
        assert!(hit.hit.path.ends_with("\\uart.c"));
        let last = batches.last().unwrap();
        assert!(last.done);
        assert_eq!((last.files_read, last.files_total), (2, 2));
    }

    #[test]
    fn a_name_match_comes_first_and_is_not_read_again() {
        let temp = Temp::new("names");
        temp.write("main.c", b"void main(void) { spi_init(); }\n");
        temp.write("spi.c", b"spi everywhere\n");
        let index = flat(&temp.0, &["main.c", "spi.c"]);
        let db = db::open_in_memory().unwrap();
        let batches = run(&db, &index, "spi", &Filters::default());
        assert_eq!(
            found(&batches),
            [("spi.c".into(), None), ("main.c".into(), Some(1))]
        );
        assert_eq!(batches.last().unwrap().files_total, 1);
    }

    #[test]
    fn binary_large_and_untyped_files_are_not_read() {
        let temp = Temp::new("skip");
        temp.write("blob.txt", b"\0\0\0 secret");
        let mut big = b"secret\n".to_vec();
        big.resize(MAX_BYTES as usize + 1, b'x');
        temp.write("big.log", &big);
        temp.write("photo.jpg", b"secret");
        temp.write("ok.md", b"the secret is here");
        let index = flat(&temp.0, &["blob.txt", "big.log", "photo.jpg", "ok.md"]);
        let db = db::open_in_memory().unwrap();
        let batches = run(&db, &index, "secret", &Filters::default());
        assert_eq!(found(&batches), [("ok.md".into(), Some(1))]);
        let last = batches.last().unwrap();
        // The JPEG is not a text type, so it was never a candidate; the skipped files
        // still count as looked at, so the progress reaches the total.
        assert_eq!((last.files_read, last.files_total), (3, 3));
    }

    #[test]
    fn the_type_filter_picks_the_files_read() {
        let temp = Temp::new("types");
        temp.write("a.c", b"token\n");
        temp.write("a.txt", b"token\n");
        temp.write("a.jpg", b"token\n");
        let index = flat(&temp.0, &["a.c", "a.txt", "a.jpg"]);
        let db = db::open_in_memory().unwrap();
        let only = |exts: &[&str]| {
            let filters = Filters {
                exts: exts.iter().map(|e| (*e).to_owned()).collect(),
                ..Filters::default()
            };
            sorted(found(&run(&db, &index, "token", &filters)))
        };
        assert_eq!(only(&["c"]), [("a.c".into(), Some(1))]);
        // A type picked by hand is read even when it is not a text type.
        assert_eq!(
            only(&["c", "jpg"]),
            [("a.c".into(), Some(1)), ("a.jpg".into(), Some(1))]
        );
    }

    #[test]
    fn every_word_must_occur_in_the_file() {
        let temp = Temp::new("words");
        temp.write("one.c", b"alpha\nand later beta\n");
        temp.write("two.c", b"alpha only\n");
        let index = flat(&temp.0, &["one.c", "two.c"]);
        let db = db::open_in_memory().unwrap();
        let batches = run(&db, &index, "beta alpha", &Filters::default());
        // The line shown is the first word's.
        assert_eq!(found(&batches), [("one.c".into(), Some(2))]);
    }

    #[test]
    fn matching_ignores_case_in_any_script() {
        let temp = Temp::new("script");
        temp.write("vn.txt", "Ảnh cưới\n".as_bytes());
        let index = flat(&temp.0, &["vn.txt"]);
        let db = db::open_in_memory().unwrap();
        let batches = run(&db, &index, "ẢNH", &Filters::default());
        assert_eq!(found(&batches), [("vn.txt".into(), Some(1))]);
    }

    #[test]
    fn a_tag_keeps_the_files_inside_its_folders() {
        let temp = Temp::new("tags");
        temp.write("proj\\x.c", b"needle\n");
        temp.write("y.c", b"needle\n");
        let index = index_of(
            &temp.0,
            &[("proj", 0, true), ("x.c", 1, false), ("y.c", 0, false)],
        );
        let db = db::open_in_memory().unwrap();
        tags::add(&db, &temp.0.join("proj").display().to_string(), "work").unwrap();
        let filters = Filters {
            tags: vec!["work".into()],
            ..Filters::default()
        };
        assert_eq!(
            found(&run(&db, &index, "needle", &filters)),
            [("x.c".into(), Some(1))]
        );
    }

    #[test]
    fn without_a_word_only_names_are_listed() {
        let temp = Temp::new("noword");
        temp.write("a.c", b"text\n");
        temp.write("b.h", b"text\n");
        let index = flat(&temp.0, &["a.c", "b.h"]);
        let db = db::open_in_memory().unwrap();
        let filters = Filters {
            exts: vec!["c".into()],
            ..Filters::default()
        };
        let batches = run(&db, &index, "", &filters);
        assert_eq!(found(&batches), [("a.c".into(), None)]);
        assert!(batches.last().unwrap().done);
    }

    #[test]
    fn a_stopped_scan_sends_nothing_more() {
        let temp = Temp::new("stop");
        let names: Vec<String> = (0..300).map(|i| format!("f{i:03}.txt")).collect();
        for name in &names {
            temp.write(name, b"hit\n");
        }
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let index = flat(&temp.0, &refs);
        let db = db::open_in_memory().unwrap();
        let query = find::query_for("hit", &Filters::default());
        let stop = AtomicBool::new(false);
        let batches = Mutex::new(Vec::new());
        scan_with(
            &db,
            &index,
            &query,
            &Filters::default(),
            &|| stop.load(Ordering::SeqCst),
            &|batch| {
                batches.lock().unwrap().push(batch);
                // A newer search starts right after the first batch.
                stop.store(true, Ordering::SeqCst);
            },
        )
        .unwrap();
        let batches = batches.into_inner().unwrap();
        assert_eq!(batches.len(), 1);
        assert!(!batches[0].done);
        assert!(found(&batches).len() < names.len());
    }

    #[test]
    fn a_long_line_is_cut_around_the_match() {
        let line = format!("{}needle{}", "a".repeat(100), "b".repeat(300));
        let at = line.find("needle").unwrap();
        let text = snippet(line.as_bytes(), at..at + 6);
        assert!(text.starts_with('…'));
        assert!(text.ends_with('…'));
        assert!(text.contains(&format!("{}needle", "a".repeat(LEAD_CHARS))));
        // The leading … counts inside the budget; the trailing one is added after it.
        assert_eq!(text.chars().count(), SNIPPET_CHARS + 1);
    }

    #[test]
    fn results_reach_the_frontend_flat_with_line_and_snippet() {
        let found = Found {
            hit: Hit {
                name: "a.c".into(),
                path: "C:\\a.c".into(),
                is_dir: false,
            },
            line: Some(3),
            snippet: Some("x".into()),
        };
        let json = serde_json::to_value(&found).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"name": "a.c", "path": "C:\\a.c", "isDir": false, "line": 3, "snippet": "x"})
        );
        let by_name = serde_json::to_value(super::by_name(found.hit)).unwrap();
        assert!(by_name.get("line").is_none());
    }
}
