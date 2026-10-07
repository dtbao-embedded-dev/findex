//! Thin `#[tauri::command]` wrappers: take the arguments, call one model function.
//! A rule written here instead of in `model/` exists twice the day a second
//! caller needs it.

use crate::error::AppResult;
use crate::model::about::{self, About};
use crate::model::db::Db;
use crate::model::greeting;
use crate::splash;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn greet(name: &str) -> AppResult<String> {
    greeting::greet(name)
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
