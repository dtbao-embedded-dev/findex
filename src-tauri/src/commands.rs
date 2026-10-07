//! Thin `#[tauri::command]` wrappers: take the arguments, call one model function.
//! A rule written here instead of in `model/` exists twice the day a second
//! caller needs it.

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::model::about::{self, About};
use crate::model::contents::{self, Batch};
use crate::model::db::Db;
use crate::model::details::{self, Details};
use crate::model::drives::{self, Drive};
use crate::model::find::{self, Answer};
use crate::model::index::service::{Emit, IndexService, Status};
use crate::model::launch;
use crate::model::reveal;
use crate::model::search::Filters;
use crate::model::tags::{self, TagInfo};
use crate::splash;
use tauri::ipc::Channel;
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

/// Reads the active drive again from scratch; progress arrives as `index-status`
/// events. Off the main thread: starting waits for the index lock.
#[tauri::command]
pub async fn rescan_drive(app: AppHandle) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        let index = app.state::<Arc<IndexService>>();
        index.rescan(&db, status_emitter(&app))
    })
    .await
    .map_err(|err| AppError::Invalid(format!("the re-scan could not start: {err}")))?
}

#[tauri::command]
pub fn index_status(index: State<'_, Arc<IndexService>>) -> AppResult<Status> {
    Ok(index.status())
}

/// Runs off the async runtime: a query is CPU work across every core, and a date
/// filter waits on the disk. Resolves to null when a newer query replaced this one.
/// Answers `count` results from rank `offset`; tags are read from the tags table.
#[tauri::command]
pub async fn search_names(
    app: AppHandle,
    query: String,
    filters: Option<Filters>,
    offset: usize,
    count: usize,
) -> AppResult<Option<Answer>> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        let index = app.state::<Arc<IndexService>>();
        find::search(
            &db,
            &index,
            &query,
            &filters.unwrap_or_default(),
            offset,
            count,
        )
    })
    .await
    .map_err(|err| AppError::Invalid(format!("the search stopped: {err}")))?
}

/// Searches names and then file contents, sending each batch of results through
/// `on_batch` as it is found; resolves once the scan ends or a newer search stopped
/// it. Off the async runtime: it reads files for as long as the drive takes.
#[tauri::command]
pub async fn search_contents(
    app: AppHandle,
    query: String,
    filters: Option<Filters>,
    on_batch: Channel<Batch>,
) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        let index = app.state::<Arc<IndexService>>();
        contents::scan(
            &db,
            &index,
            &query,
            &filters.unwrap_or_default(),
            &|batch| {
                if let Err(err) = on_batch.send(batch) {
                    eprintln!("could not send content results: {err}");
                }
            },
        )
    })
    .await
    .map_err(|err| AppError::Invalid(format!("the content search stopped: {err}")))?
}

/// Stops the search running in Rust, such as a content scan the user left.
#[tauri::command]
pub fn cancel_search(index: State<'_, Arc<IndexService>>) -> AppResult<()> {
    index.cancel_searches();
    Ok(())
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

/// Every tag that can be picked: the presets, then custom tags.
#[tauri::command]
pub fn tag_catalog(db: State<'_, Db>) -> AppResult<Vec<TagInfo>> {
    tags::catalog(&db)
}

#[tauri::command]
pub fn create_tag(db: State<'_, Db>, tag: &str) -> AppResult<Vec<TagInfo>> {
    tags::create(&db, tag)
}

/// Deletes a custom tag and takes it off every folder.
#[tauri::command]
pub fn delete_tag(db: State<'_, Db>, tag: &str) -> AppResult<Vec<TagInfo>> {
    tags::delete(&db, tag)
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
