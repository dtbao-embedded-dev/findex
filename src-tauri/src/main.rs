// Keep the extra console window from appearing behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(err) = findex_lib::run() {
        eprintln!("findex failed to start: {err}");
        std::process::exit(1);
    }
}
