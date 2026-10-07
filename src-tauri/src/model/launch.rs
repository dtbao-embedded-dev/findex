//! Opens a folder in Explorer, in a terminal, or in Claude Code. Findex runs as
//! administrator, so a terminal it started would be too; terminals and Claude start
//! as the signed-in user instead, through `win32::spawn_as_shell_user`.
//!
//! The folder is never put on a command line: it becomes the new process's working
//! directory, so a name holding spaces, quotes or `;` (which Windows Terminal treats
//! as a command separator) cannot change what runs.

use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AppError, AppResult};
use crate::model::win32;

/// The terminal a folder opens in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminal {
    /// Windows Terminal, found at this path.
    WindowsTerminal(PathBuf),
    /// PowerShell in a console window, always present.
    PowerShell,
}

/// Opens `path` in a File Explorer window. Explorer hands the window to the running
/// shell, which is never elevated.
pub fn open_folder(path: &str) -> AppResult<()> {
    let dir = folder(path)?;
    Command::new("explorer.exe")
        .raw_arg(explorer_arg(&dir.to_string_lossy()))
        .spawn()?;
    Ok(())
}

/// Opens a terminal working in `path`.
pub fn open_terminal(path: &str) -> AppResult<()> {
    let dir = folder(path)?;
    start(&terminal_command(&find_terminal(), None), &dir)
}

/// Opens a terminal working in `path` and runs Claude Code (`claude`) there. The
/// program found on PATH is run by its full path: the folder is untrusted, and a
/// `claude.cmd` in it would otherwise run first.
pub fn open_claude(path: &str) -> AppResult<()> {
    let dir = folder(path)?;
    let claude = find_on_path("claude", &env("PATH"), &env("PATHEXT")).ok_or_else(|| {
        AppError::Invalid("Claude Code is not installed (claude not found on PATH)".into())
    })?;
    start(&terminal_command(&find_terminal(), Some(&claude)), &dir)
}

/// The command line that opens `terminal` in its working directory, running the
/// program at `run` there when given. Without Windows Terminal, `cmd /k` keeps the
/// console open after it exits; with `/s` it strips exactly the outer pair of quotes,
/// so the quoted path survives spaces and parentheses.
pub fn terminal_command(terminal: &Terminal, run: Option<&Path>) -> String {
    match (terminal, run) {
        (Terminal::WindowsTerminal(wt), None) => format!("\"{}\" -d .", wt.display()),
        // Windows Terminal splits its arguments and quotes them again itself, so the
        // program goes in as one quoted argument; it runs a `.cmd` shim as well.
        (Terminal::WindowsTerminal(wt), Some(program)) => {
            format!("\"{}\" -d . \"{}\"", wt.display(), program.display())
        }
        (Terminal::PowerShell, None) => "powershell.exe -NoExit".into(),
        (Terminal::PowerShell, Some(program)) => {
            format!("cmd.exe /s /k \"\"{}\"\"", program.display())
        }
    }
}

/// The child's environment: Findex's own, plus `NoDefaultCurrentDirectoryInExePath`,
/// so neither cmd nor a shim it runs (npm's `claude.cmd` calls a bare `node`) picks a
/// program out of the untrusted folder. Sorted by name, as CreateProcessW expects.
fn environment_block(vars: Vec<(String, String)>) -> Vec<u16> {
    const NO_CWD: &str = "NoDefaultCurrentDirectoryInExePath";
    let mut vars: Vec<(String, String)> = vars
        .into_iter()
        .filter(|(name, _)| !name.eq_ignore_ascii_case(NO_CWD))
        .chain(std::iter::once((NO_CWD.to_owned(), "1".to_owned())))
        .collect();
    vars.sort_by_key(|(name, _)| name.to_uppercase());
    let mut block: Vec<u16> = vars
        .iter()
        .flat_map(|(name, value)| {
            format!("{name}={value}\0")
                .encode_utf16()
                .collect::<Vec<_>>()
        })
        .collect();
    block.push(0);
    block
}

/// The first `name` on `path`, trying each extension of `pathext` when `name` has
/// none, as the command prompt does. An app execution alias (how Windows Terminal's
/// `wt.exe` is installed) counts, though it cannot be opened as a plain file.
pub fn find_on_path(name: &str, path: &str, pathext: &str) -> Option<PathBuf> {
    let names: Vec<String> = if Path::new(name).extension().is_some() {
        vec![name.to_owned()]
    } else {
        pathext
            .split(';')
            .filter(|ext| !ext.is_empty())
            .map(|ext| format!("{name}{}", ext.to_ascii_lowercase()))
            .collect()
    };
    path.split(';')
        .filter(|dir| !dir.trim().is_empty())
        .flat_map(|dir| names.iter().map(move |n| Path::new(dir.trim()).join(n)))
        .find(|candidate| std::fs::symlink_metadata(candidate).is_ok_and(|meta| !meta.is_dir()))
}

/// Explorer parses its own command line: a quoted root such as `"C:\"` would end in
/// an escaped quote, so a trailing backslash gets a `.` after it.
fn explorer_arg(dir: &str) -> String {
    if dir.ends_with('\\') {
        format!("\"{dir}.\"")
    } else {
        format!("\"{dir}\"")
    }
}

fn find_terminal() -> Terminal {
    find_on_path("wt.exe", &env("PATH"), &env("PATHEXT"))
        .map_or(Terminal::PowerShell, Terminal::WindowsTerminal)
}

/// Starts `command_line` in `dir` as the signed-in user.
fn start(command_line: &str, dir: &Path) -> AppResult<()> {
    let env = environment_block(
        std::env::vars_os()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
            .collect(),
    );
    start_with(
        &|as_user| win32::create_process(command_line, dir, as_user, &env),
        win32::elevation(),
    )
}

/// Tries `create` as the signed-in user first. Unelevated, a plain start already runs
/// as the user, so it is the fallback. Elevated, or when that cannot be told, failing
/// is an error: a terminal must never inherit administrator rights by surprise.
fn start_with(create: &dyn Fn(bool) -> io::Result<()>, elevated: Option<bool>) -> AppResult<()> {
    match create(true) {
        Ok(()) => Ok(()),
        Err(_) if elevated == Some(false) => Ok(create(false)?),
        Err(err) => Err(AppError::Invalid(format!(
            "could not start it with your normal rights: {err}"
        ))),
    }
}

/// `path` as a folder that exists; a result deleted since it was indexed is refused.
fn folder(path: &str) -> AppResult<PathBuf> {
    let dir = PathBuf::from(path);
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err(AppError::Invalid(format!(
            "{path} is not a folder that exists"
        )))
    }
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_terminal_opens_in_its_working_directory() {
        let wt = Terminal::WindowsTerminal(PathBuf::from(r"C:\Program Files\WT\wt.exe"));
        let claude = Path::new(r"C:\Users\a b\.local\bin\claude.exe");
        assert_eq!(
            terminal_command(&wt, None),
            r#""C:\Program Files\WT\wt.exe" -d ."#
        );
        // The program found on PATH is named in full: a `claude.cmd` planted in the
        // folder must never run in its place. Windows Terminal parses its own
        // arguments and quotes them again, so the path is one plain quoted argument.
        assert_eq!(
            terminal_command(&wt, Some(claude)),
            r#""C:\Program Files\WT\wt.exe" -d . "C:\Users\a b\.local\bin\claude.exe""#
        );
    }

    #[test]
    fn powershell_is_the_fallback() {
        let claude = Path::new(r"C:\bin\claude.cmd");
        assert_eq!(
            terminal_command(&Terminal::PowerShell, None),
            "powershell.exe -NoExit"
        );
        assert_eq!(
            terminal_command(&Terminal::PowerShell, Some(claude)),
            r#"cmd.exe /s /k ""C:\bin\claude.cmd"""#
        );
    }

    #[test]
    fn the_child_environment_never_searches_the_folder_for_programs() {
        let block = environment_block(vec![
            ("Path".into(), r"C:\bin".into()),
            ("ComSpec".into(), "cmd".into()),
        ]);
        let text = String::from_utf16(&block).unwrap();
        assert_eq!(
            text,
            "ComSpec=cmd\0NoDefaultCurrentDirectoryInExePath=1\0Path=C:\\bin\0\0"
        );
    }

    #[test]
    fn starts_unelevated_and_never_falls_back_to_admin_rights() {
        use std::io;
        let tries = std::cell::RefCell::new(Vec::new());
        let fails_as_user = |as_user: bool| {
            tries.borrow_mut().push(as_user);
            if as_user {
                Err(io::Error::other("no shell"))
            } else {
                Ok(())
            }
        };
        // Started as the user: done.
        assert!(start_with(&|_| Ok(()), Some(true)).is_ok());
        // Elevated, or elevation unknown: refuse rather than start with admin rights.
        assert!(matches!(
            start_with(&fails_as_user, Some(true)),
            Err(AppError::Invalid(_))
        ));
        assert!(matches!(
            start_with(&fails_as_user, None),
            Err(AppError::Invalid(_))
        ));
        assert_eq!(*tries.borrow(), [true, true]);
        // Not elevated: a plain start already runs as the user.
        assert!(start_with(&fails_as_user, Some(false)).is_ok());
        assert_eq!(*tries.borrow(), [true, true, true, false]);
    }

    #[test]
    fn finds_a_program_by_its_extensions_in_path_order() {
        let root = std::env::temp_dir().join(format!("findex-path-{}", std::process::id()));
        let (first, second) = (root.join("a b"), root.join("c"));
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(second.join("claude.cmd"), "").unwrap();
        std::fs::write(first.join("claude.exe"), "").unwrap();
        let path = format!("{};;{}", first.display(), second.display());

        let found = find_on_path("claude", &path, ".COM;.EXE;.CMD");
        assert_eq!(found, Some(first.join("claude.exe")));
        let found = find_on_path("claude", &second.display().to_string(), ".COM;.EXE;.CMD");
        assert_eq!(found, Some(second.join("claude.cmd")));
        assert_eq!(
            find_on_path("claude.cmd", &path, ".EXE"),
            Some(second.join("claude.cmd"))
        );
        assert_eq!(find_on_path("missing", &path, ".EXE"), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Needs a desktop session and Windows Terminal; opens and closes two windows.
    #[test]
    #[ignore]
    fn a_program_in_a_spaced_path_runs_in_the_folder_through_either_terminal() {
        let root = std::env::temp_dir().join(format!("findex-run-{}", std::process::id()));
        let bin = root.join("a b (x)");
        let work = root.join("work; dir");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&work).unwrap();
        let program = bin.join("probe.cmd");
        std::fs::write(&program, "@echo %CD%> \"%~dp0where.txt\"\r\n@exit\r\n").unwrap();
        let wt = find_on_path("wt.exe", &env("PATH"), &env("PATHEXT")).unwrap();
        for terminal in [Terminal::WindowsTerminal(wt), Terminal::PowerShell] {
            let written = bin.join("where.txt");
            let _ = std::fs::remove_file(&written);
            let command = terminal_command(&terminal, Some(&program));
            win32::create_process(&command, &work, true, &[]).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            while !written.exists() && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
            let text = std::fs::read_to_string(&written).unwrap();
            assert_eq!(text.trim(), work.display().to_string(), "{terminal:?}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_root_folder_keeps_its_closing_quote() {
        assert_eq!(explorer_arg(r"C:\"), r#""C:\.""#);
        assert_eq!(explorer_arg(r"E:\a b;c"), r#""E:\a b;c""#);
    }

    #[test]
    fn refuses_a_path_that_is_not_a_folder() {
        let file = std::env::current_exe().unwrap();
        assert!(matches!(
            folder(&file.display().to_string()),
            Err(AppError::Invalid(_))
        ));
        assert!(matches!(
            folder(r"Z:\no\such\folder"),
            Err(AppError::Invalid(_))
        ));
        let dir = std::env::temp_dir();
        assert_eq!(folder(&dir.display().to_string()).unwrap(), dir);
    }
}
