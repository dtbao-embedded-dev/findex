//! Thin `#[tauri::command]` wrappers: take the arguments, call one model function.
//! A rule written here instead of in `model/` exists twice the day a second
//! caller needs it.

use crate::error::AppResult;
use crate::model::db::Db;
use crate::model::greeting;
use tauri::State;

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
