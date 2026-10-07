//! Opens a folder in Explorer, in a terminal, or in Claude Code. Findex runs as
//! administrator, so a terminal it started would be too; terminals and Claude start
//! as the signed-in user instead, through `win32::spawn_as_shell_user`.
//!
//! The folder is never put on a command line: it becomes the new process's working
//! directory, so a name holding spaces, quotes or `;` (which Windows Terminal treats
//! as a command separator) cannot change what runs.

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

/// Opens a terminal working in `path` and runs Claude Code (`claude`) there.
pub fn open_claude(path: &str) -> AppResult<()> {
    let dir = folder(path)?;
    if find_on_path("claude", &env("PATH"), &env("PATHEXT")).is_none() {
        return Err(AppError::Invalid(
            "Claude Code is not installed (claude not found on PATH)".into(),
        ));
    }
    start(&terminal_command(&find_terminal(), Some("claude")), &dir)
}

/// The command line that opens `terminal` in its working directory, running `run`
/// there when given. `cmd /k` keeps the window open after `run` exits, and finds a
/// `.cmd` shim as well as an `.exe`.
pub fn terminal_command(terminal: &Terminal, run: Option<&str>) -> String {
    match (terminal, run) {
        (Terminal::WindowsTerminal(wt), None) => format!("\"{}\" -d .", wt.display()),
        (Terminal::WindowsTerminal(wt), Some(run)) => {
            format!("\"{}\" -d . cmd.exe /k {run}", wt.display())
        }
        (Terminal::PowerShell, None) => "powershell.exe -NoExit".into(),
        (Terminal::PowerShell, Some(run)) => format!("cmd.exe /k {run}"),
    }
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

/// Starts as the signed-in user. Elevated, failing that is an error: a terminal
/// must never inherit administrator rights by surprise. Unelevated, a plain start
/// already runs as the user.
fn start(command_line: &str, dir: &Path) -> AppResult<()> {
    match win32::create_process(command_line, dir, true) {
        Ok(()) => Ok(()),
        Err(_) if !win32::is_elevated() => Ok(win32::create_process(command_line, dir, false)?),
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
        assert_eq!(
            terminal_command(&wt, None),
            r#""C:\Program Files\WT\wt.exe" -d ."#
        );
        assert_eq!(
            terminal_command(&wt, Some("claude")),
            r#""C:\Program Files\WT\wt.exe" -d . cmd.exe /k claude"#
        );
    }

    #[test]
    fn powershell_is_the_fallback() {
        assert_eq!(
            terminal_command(&Terminal::PowerShell, None),
            "powershell.exe -NoExit"
        );
        assert_eq!(
            terminal_command(&Terminal::PowerShell, Some("claude")),
            "cmd.exe /k claude"
        );
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
