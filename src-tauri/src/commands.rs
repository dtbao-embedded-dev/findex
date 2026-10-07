//! Thin `#[tauri::command]` wrappers: take the arguments, call one model function.
//! A rule written here instead of in `model/` exists twice the day a second
//! caller needs it.

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::model::about::{self, About};
use crate::model::db::Db;
use crate::model::details::{self, Details};
use crate::model::drives::{self, Drive};
use crate::model::find::{self, Answer};
use crate::model::index::service::{Emit, IndexService, Status};
use crate::model::launch;
use crate::model::reveal;
use crate::model::search::{Filters, DEFAULT_LIMIT};
use crate::model::tags::{self, TagCount};
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

/// The drive being indexed and searched; null only when no drive exists.
#[tauri::command]
pub fn get_active_drive(db: State<'_, Db>) -> AppResult<Option<String>> {
    drives::load_active(&db)
}

/// Saves `letter` as the active drive and indexes it alone; progress arrives as
/// `index-status` events. Off the main thread: starting waits for the index lock,
/// which a snapshot save can hold for a moment.
#[tauri::command]
pub async fn set_active_drive(app: AppHandle, letter: String) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        let index = app.state::<Arc<IndexService>>();
        index.switch_to(&db, &letter, status_emitter(&app))
    })
    .await
    .map_err(|err| AppError::Invalid(format!("the drive could not be switched: {err}")))?
}

/// Saves the drive index changed by live updates. The updater's installer ends the
/// app with `std::process::exit`, which skips the save `lib.rs` does on quit.
#[tauri::command]
pub async fn save_index(index: State<'_, Arc<IndexService>>) -> AppResult<()> {
    let index = Arc::clone(&index);
    tauri::async_runtime::spawn_blocking(move || index.save_changed())
        .await
        .map_err(|err| AppError::Invalid(format!("saving the index stopped: {err}")))
}

#[tauri::command]
pub fn index_status(index: State<'_, Arc<IndexService>>) -> AppResult<Status> {
    Ok(index.status())
}

/// Runs off the async runtime: a query is CPU work across every core, and a date
/// filter waits on the disk. Resolves to null when a newer query replaced this one.
/// Tags are read from the tags table.
#[tauri::command]
pub async fn search_names(
    app: AppHandle,
    query: String,
    filters: Option<Filters>,
    limit: Option<usize>,
) -> AppResult<Option<Answer>> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        let index = app.state::<Arc<IndexService>>();
        let filters = filters.unwrap_or_default();
        find::search(
            &db,
            &index,
            &query,
            &filters,
            limit.unwrap_or(DEFAULT_LIMIT),
        )
    })
    .await
    .map_err(|err| AppError::Invalid(format!("the search stopped: {err}")))?
}

/// Type, size, date and tags for the results on screen. Off the async runtime: it
/// waits on the disk.
#[tauri::command]
pub async fn describe_paths(app: AppHandle, paths: Vec<String>) -> AppResult<Vec<Details>> {
    tauri::async_runtime::spawn_blocking(move || {
        details::describe_tagged(&app.state::<Db>(), &paths)
    })
    .await
    .map_err(|err| AppError::Invalid(format!("reading file details stopped: {err}")))?
}

#[tauri::command]
pub fn add_tag(db: State<'_, Db>, path: &str, tag: &str) -> AppResult<Vec<String>> {
    tags::add(&db, path, tag)
}

#[tauri::command]
pub fn remove_tag(db: State<'_, Db>, path: &str, tag: &str) -> AppResult<Vec<String>> {
    tags::remove(&db, path, tag)
}

#[tauri::command]
pub fn list_tags(db: State<'_, Db>) -> AppResult<Vec<TagCount>> {
    tags::all(&db)
}

#[tauri::command]
pub fn reveal_path(path: &str) -> AppResult<()> {
    reveal::reveal(path)
}

#[tauri::command]
pub fn open_folder(path: &str) -> AppResult<()> {
    launch::open_folder(path)
}

#[tauri::command]
pub fn open_terminal(path: &str) -> AppResult<()> {
    launch::open_terminal(path)
}

#[tauri::command]
pub fn open_claude(path: &str) -> AppResult<()> {
    launch::open_claude(path)
}
