//! `findex-tag`: what Explorer's "Tag with Findex" menu runs, so a folder can be
//! tagged while Findex stays in the tray.
//!
//! Findex runs as administrator, and `tauri-build` embeds that manifest in every
//! binary of the `findex` crate, so this is a crate of its own that runs with the
//! user's normal rights. It never opens Findex's database: it drops a request into
//! Findex's inbox folder, which the app applies within a second, or when it next
//! starts.
//!
//! Usage: `findex-tag --inbox <dir> --tag <tag> <folder>`
//!
//! A request is a `.req` file holding the tag on its first line and the folder on
//! its second. It is written as `.tmp` and renamed, so the app never reads half of
//! one.

#![windows_subsystem = "windows"]

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

/// The longest tag, in characters, as Findex stores them.
const MAX_TAG: usize = 32;

/// One folder to tag, and where Findex picks the request up.
#[derive(Debug, PartialEq, Eq)]
struct Request {
    inbox: PathBuf,
    tag: String,
    folder: String,
}

fn main() -> ExitCode {
    // No console to print to: the exit code is all Explorer would see.
    match parse(std::env::args().skip(1))
        .and_then(|request| write_request(&request, &unique_name()).map_err(|err| err.to_string()))
    {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// Reads `--inbox <dir> --tag <tag> <folder>`, in any order of the two options.
fn parse(args: impl IntoIterator<Item = String>) -> Result<Request, String> {
    let mut inbox = None;
    let mut tag = None;
    let mut folder = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--inbox" => inbox = args.next(),
            "--tag" => tag = args.next(),
            _ if folder.is_none() => folder = Some(arg),
            _ => return Err(format!("unexpected argument: {arg}")),
        }
    }
    let inbox = inbox
        .filter(|dir| !dir.is_empty())
        .ok_or("--inbox is missing")?;
    let tag = normalize_tag(&tag.ok_or("--tag is missing")?)?;
    // Explorer quotes the folder as `"%V"`; for a drive root that is `"E:\"`, whose
    // `\"` the command line reads as a literal quote, giving `E:"`. A quote can
    // never be part of a Windows path, so a trailing one is that backslash.
    let folder = folder
        .map(|path| match path.trim().strip_suffix('"') {
            Some(root) => format!("{}\\", root.trim_end_matches('"')),
            None => path.trim().to_owned(),
        })
        .filter(|path| !path.is_empty() && !path.contains(['\r', '\n']))
        .ok_or("the folder is missing")?;
    Ok(Request {
        inbox: PathBuf::from(inbox),
        tag,
        folder,
    })
}

/// The same rule Findex applies (`model/tags.rs`): trimmed, no leading `#`, lower
/// case, 1 to 32 letters, digits, `-` or `_`. Findex checks again when it applies it.
fn normalize_tag(tag: &str) -> Result<String, String> {
    let tag = tag.trim().trim_start_matches('#').to_lowercase();
    let valid = !tag.is_empty()
        && tag.chars().count() <= MAX_TAG
        && tag
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(tag)
    } else {
        Err(format!("not a tag: {tag}"))
    }
}

/// Writes the request as `<name>.tmp`, then renames it to `<name>.req`.
fn write_request(request: &Request, name: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(&request.inbox)?;
    let partial = request.inbox.join(format!("{name}.tmp"));
    let done = request.inbox.join(format!("{name}.req"));
    std::fs::write(&partial, format!("{}\n{}\n", request.tag, request.folder))?;
    if let Err(err) = std::fs::rename(&partial, &done) {
        let _ = std::fs::remove_file(&partial);
        return Err(err);
    }
    Ok(done)
}

/// Milliseconds and process id: two clicks never share a name.
fn unique_name() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    format!("{ms}-{}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn reads_the_inbox_the_tag_and_the_folder() {
        let request = parse(args(&["--inbox", r"C:\in", "--tag", "#Working", r"E:\a b"])).unwrap();
        assert_eq!(
            request,
            Request {
                inbox: PathBuf::from(r"C:\in"),
                tag: "working".into(),
                folder: r"E:\a b".into(),
            }
        );
        // The folder may come first: Explorer appends it after the options anyway.
        let first = parse(args(&[r"E:\x", "--tag", "doc", "--inbox", r"C:\in"])).unwrap();
        assert_eq!(first.folder, r"E:\x");
    }

    #[test]
    fn a_drive_root_keeps_its_backslash() {
        // `"E:\"` on the command line arrives as `E:"`.
        let root = parse(args(&["--inbox", "i", "--tag", "doc", "E:\""])).unwrap();
        assert_eq!(root.folder, r"E:\");
    }

    #[test]
    fn refuses_a_bad_tag_or_a_missing_part() {
        assert!(parse(args(&["--inbox", "i", "--tag", "two words", "f"])).is_err());
        assert!(parse(args(&["--inbox", "i", "--tag", "", "f"])).is_err());
        assert!(parse(args(&["--inbox", "i", "--tag", &"x".repeat(33), "f"])).is_err());
        assert!(parse(args(&["--tag", "doc", "f"])).is_err());
        assert!(parse(args(&["--inbox", "i", "f"])).is_err());
        assert!(parse(args(&["--inbox", "i", "--tag", "doc"])).is_err());
        assert!(parse(args(&["--inbox", "i", "--tag", "doc", "f", "extra"])).is_err());
        assert!(parse(args(&["--inbox", "i", "--tag", "doc", "a\nb"])).is_err());
    }

    #[test]
    fn writes_one_complete_request_and_no_partial_file() {
        let inbox = std::env::temp_dir().join(format!("findex-tag-{}", std::process::id()));
        let request = Request {
            inbox: inbox.clone(),
            tag: "doc".into(),
            folder: r"E:\Baotd\docs".into(),
        };
        let written = write_request(&request, "1-2").unwrap();
        assert_eq!(written, inbox.join("1-2.req"));
        assert_eq!(
            std::fs::read_to_string(&written).unwrap(),
            "doc\nE:\\Baotd\\docs\n"
        );
        let names: Vec<_> = std::fs::read_dir(&inbox)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["1-2.req"]);
        std::fs::remove_dir_all(inbox).unwrap();
    }
}
