//! Application entry: plugins, state and the command table. No business rules
//! live here — they belong in `model/`, reached through `commands`.

mod commands;
mod error;
mod model;
mod splash;
mod tray;

use std::sync::Arc;

use tauri::Manager;

use model::index::service::IndexService;

/// Builds and runs the application. `main.rs` turns an error into an exit code.
pub fn run() -> tauri::Result<()> {
    // Before anything else, so the single-instance plugin sees only the copy that
    // keeps running.
    if model::elevation::relaunch_if_needed() {
        return Ok(());
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            tray::init(app)?;
            let data = app.path().app_data_dir()?;
            let db = model::db::open(&data.join("findex.sqlite3"))?;
            let index = Arc::new(IndexService::with_snapshots(data.join("index")));
            index.start_saved(&db, commands::status_emitter(app.handle()))?;
            app.manage(db);
            app.manage(index);
            splash::arm_fallback(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing main hides to the tray; the tray menu's Quit ends the app.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                api.prevent_close();
                if let Err(err) = window.hide() {
                    eprintln!("could not hide the window: {err}");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_setting,
            commands::set_setting,
            commands::app_about,
            commands::splash_ready,
            commands::list_drives,
            commands::get_scan_drives,
            commands::set_scan_drives,
            commands::start_index,
            commands::index_status,
            commands::search_names,
            commands::reveal_path
        ])
        .build(tauri::generate_context!())?
        .run(|app, event| {
            // Live updates since the last save would otherwise be replayed from the
            // journal, or lost to a rebuild if it wrapped meanwhile.
            if let tauri::RunEvent::Exit = event {
                app.state::<Arc<IndexService>>().save_changed();
            }
        });
    Ok(())
}
