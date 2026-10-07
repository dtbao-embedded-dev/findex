//! The splash window shown while the hidden main window loads. The main frontend ends
//! it once it is ready; a timer ends it anyway, so a frontend that fails to load never
//! leaves the app with no visible window.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};

use crate::tray;

/// How long the splash may stay up before main is shown regardless.
const FALLBACK: Duration = Duration::from_secs(10);

/// Set by the first `finish`. Later calls (the fallback timer) must not re-show a main
/// window the user has since hidden to the tray. Not tied to the splash window still
/// existing: the user may close it with Alt+F4 before main is ready.
static FINISHED: AtomicBool = AtomicBool::new(false);

/// Destroys the splash and shows main, once.
pub fn finish<R: Runtime>(app: &AppHandle<R>) {
    if FINISHED.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(splash) = app.get_webview_window("splash") {
        if let Err(err) = splash.destroy() {
            eprintln!("could not close the splash window: {err}");
        }
    }
    tray::show_main(app);
}

/// Starts the timer that ends the splash if the frontend never does.
pub fn arm_fallback<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FALLBACK);
        finish(&app);
    });
}
