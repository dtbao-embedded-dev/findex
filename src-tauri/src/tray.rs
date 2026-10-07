//! The tray icon and its menu. Closing the window hides it to the tray (see
//! `lib.rs`); the menu's Quit is what ends the app. "Check for updates" asks the
//! frontend, which runs the update flow, to check now.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, Runtime};

/// Shows, unminimizes and focuses the main window.
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        // Each call fails only when the window is already gone: nothing to recover.
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Shows the window and asks the frontend to check for an update now; the answer
/// appears there.
fn check_for_updates<R: Runtime>(app: &AppHandle<R>) {
    show_main(app);
    if let Err(err) = app.emit("check-updates", ()) {
        eprintln!("could not ask for an update check: {err}");
    }
}

/// Builds the tray icon: a left click shows the window; the menu offers Show, Check
/// for updates and Quit.
pub fn init(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "Check for updates", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &check, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Findex")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "check" => check_for_updates(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
