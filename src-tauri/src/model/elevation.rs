//! Asks for administrator rights when the app opens: reading a drive's file table,
//! the fast way to index it, needs them. Release builds already ask through their
//! manifest; this covers a build started without one, by relaunching through UAC.
//!
//! `tauri dev` is left alone: it treats the first process exiting as the app
//! closing and stops the dev server, so the relaunched window would stay blank.
//! There, start the terminal as administrator instead.

use crate::model::win32;

/// Added to the relaunched copy's arguments, so a copy that still is not elevated
/// never asks again in a loop.
const ASKED: &str = "--elevated";

/// Whether to relaunch through UAC.
pub fn should_relaunch(elevated: bool, under_tauri_dev: bool, already_asked: bool) -> bool {
    !elevated && !under_tauri_dev && !already_asked
}

/// `tauri dev` starts the app through `cargo run`, which sets `CARGO`, and the CLI
/// exports `TAURI_ENV_DEBUG` to everything it starts.
pub fn under_tauri_dev(is_set: impl Fn(&str) -> bool) -> bool {
    is_set("TAURI_ENV_DEBUG") || is_set("CARGO")
}

/// Relaunches through UAC when needed. True when an elevated copy started and this
/// process should exit; false to carry on as is (already elevated, under `tauri dev`,
/// or the user said No).
pub fn relaunch_if_needed() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let asked = args.iter().any(|arg| arg == ASKED);
    let dev = under_tauri_dev(|name| std::env::var_os(name).is_some());
    if !should_relaunch(win32::is_elevated(), dev, asked) {
        return false;
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    win32::run_elevated(&exe, &command_line(&args))
}

/// The relaunched copy's arguments: the original ones, quoted, plus the marker.
fn command_line(args: &[String]) -> String {
    args.iter()
        .map(|arg| format!("\"{}\"", arg.replace('"', "\\\"")))
        .chain(std::iter::once(ASKED.to_owned()))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relaunches_only_an_unelevated_app_outside_tauri_dev() {
        assert!(should_relaunch(false, false, false));
        assert!(!should_relaunch(true, false, false));
        assert!(!should_relaunch(false, true, false));
        // A copy that was already relaunched never asks again.
        assert!(!should_relaunch(false, false, true));
    }

    #[test]
    fn recognises_tauri_dev_by_its_environment() {
        assert!(under_tauri_dev(|name| name == "CARGO"));
        assert!(under_tauri_dev(|name| name == "TAURI_ENV_DEBUG"));
        assert!(!under_tauri_dev(|_| false));
    }

    #[test]
    fn keeps_the_original_arguments_and_adds_the_marker() {
        let args = vec!["C:\\a b".to_owned(), "x".to_owned()];
        assert_eq!(command_line(&args), "\"C:\\a b\" \"x\" --elevated");
        assert_eq!(command_line(&[]), "--elevated");
    }
}
