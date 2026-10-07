//! Thin `#[tauri::command]` wrappers: take the arguments, call one model function.
//! A rule written here instead of in `model/` exists twice the day a second
//! caller needs it.

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::model::about::{self, About};
use crate::model::db::Db;
use crate::model::drives::{self, Drive};
use crate::model::index::service::{Emit, IndexService, Status};
use crate::model::reveal;
use crate::model::search::{Hit, DEFAULT_LIMIT};
use crate::splash;
use tauri::{AppHandle, Emitter, Manager, State};

/// Forwards index progress to the frontend as the `index-status` event.
pub fn status_emitter(app: &AppHandle) -> Emit {
    let app = app.clone();
    Arc::new(move |status: &Status| {
        if let Err(err) = app.emit("index-status", status) {
            eprintln!("could not send the index status: {err}");
        }
    })
}

#[tauri::command]
pub fn get_setting(db: State<'_, Db>, key: &str) -> AppResult<Option<String>> {
    db.get_setting(key)
}

#[tauri::command]
pub fn set_setting(db: State<'_, Db>, key: &str, value: &str) -> AppResult<()> {
    db.set_setting(key, value)
}

#[tauri::command]
pub fn app_about(app: AppHandle) -> AppResult<About> {
    let info = app.package_info();
    // Set by build.rs from tauri.conf.json: the runtime config drops the copyright.
    let copyright = option_env!("FINDEX_COPYRIGHT");
    Ok(about::about(
        &info.name,
        &info.version.to_string(),
        copyright,
    ))
}

/// The main frontend calls this once it can be shown.
#[tauri::command]
pub fn splash_ready(app: AppHandle) -> AppResult<()> {
    splash::finish(&app);
    Ok(())
}

/// Off the main thread: probing a sleeping USB disk can take seconds.
#[tauri::command]
pub async fn list_drives() -> AppResult<Vec<Drive>> {
    tauri::async_runtime::spawn_blocking(drives::list)
        .await
        .map_err(|err| AppError::Invalid(format!("listing drives stopped: {err}")))
}

/// The drives chosen in setup; null until setup has been done.
#[tauri::command]
pub fn get_scan_drives(db: State<'_, Db>) -> AppResult<Option<Vec<String>>> {
    drives::load_selection(&db)
}

#[tauri::command]
pub fn set_scan_drives(db: State<'_, Db>, letters: Vec<String>) -> AppResult<Vec<String>> {
    drives::save_selection(&db, &letters)
}

/// (Re)indexes the drives saved in setup; progress arrives as `index-status` events.
/// Off the main thread: starting waits for the index lock, which a snapshot save
/// can hold for a moment.
#[tauri::command]
pub async fn start_index(app: AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        let index = app.state::<Arc<IndexService>>();
        index.start_saved(&db, status_emitter(&app))
    })
    .await
    .map_err(|err| AppError::Invalid(format!("indexing could not start: {err}")))?
}

#[tauri::command]
pub fn index_status(index: State<'_, Arc<IndexService>>) -> AppResult<Status> {
    Ok(index.status())
}

/// Runs off the async runtime: a query is CPU work across every core. Resolves to
/// null when a newer query replaced this one.
#[tauri::command]
pub async fn search_names(
    index: State<'_, Arc<IndexService>>,
    query: String,
    limit: Option<usize>,
) -> AppResult<Option<Vec<Hit>>> {
    let index = Arc::clone(&index);
    tauri::async_runtime::spawn_blocking(move || {
        index.search(&query, limit.unwrap_or(DEFAULT_LIMIT))
    })
    .await
    .map_err(|err| AppError::Invalid(format!("the search stopped: {err}")))
}

#[tauri::command]
pub fn reveal_path(path: &str) -> AppResult<()> {
    reveal::reveal(path)
}
