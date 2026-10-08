//! "Tag with Findex" in Explorer's right-click menu, on a folder and on the empty
//! space of an open folder: a submenu with one entry per tag. Each entry runs
//! `findex-tag.exe`, which runs with the user's normal rights and leaves a request
//! in the inbox (see `inbox.rs`), so tagging works while Findex sits in the tray.
//!
//! The menu is a set of static verbs under `HKEY_CURRENT_USER\Software\Classes`, so
//! it needs no shell extension; Windows 11 lists it under "Show more options". It
//! is rewritten whenever the tags change, and removed when turned off in the
//! settings or when Findex is uninstalled (`windows/hooks.nsh`).

use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::model::db::Db;
use crate::model::tags::{self, TagInfo};
use crate::model::win32;

/// The setting that turns the menu off ("off"); on unless set.
pub const SETTING: &str = "explorer.menu";

/// The menu's two homes: a folder, and the background of an open folder.
pub const ROOTS: [&str; 2] = [
    r"Software\Classes\Directory\shell\FindexTag",
    r"Software\Classes\Directory\Background\shell\FindexTag",
];

/// A static submenu shows at most 16 entries.
const MAX_ITEMS: usize = 16;

/// One registry string to write: `name` `None` is the key's default value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegValue {
    pub key: String,
    pub name: Option<String>,
    pub value: String,
}

/// Whether the menu should be there: on unless the setting says "off".
pub fn enabled(db: &Db) -> AppResult<bool> {
    Ok(db.get_setting(SETTING)?.as_deref() != Some("off"))
}

/// Saves the choice, then puts the menu in place or takes it away.
pub fn set_enabled(db: &Db, on: bool, helper: &Path, icon: &Path, inbox: &Path) -> AppResult<()> {
    db.set_setting(SETTING, if on { "on" } else { "off" })?;
    sync(db, helper, icon, inbox)
}

/// Makes the registry match the setting and the tags: the old menu is removed
/// first, so a deleted tag never lingers. A missing helper (a build without it)
/// leaves no menu, since every entry would fail.
pub fn sync(db: &Db, helper: &Path, icon: &Path, inbox: &Path) -> AppResult<()> {
    for root in ROOTS {
        win32::delete_user_tree(root)?;
    }
    if !enabled(db)? {
        return Ok(());
    }
    if !helper.is_file() {
        return Err(AppError::Invalid(format!(
            "the Explorer menu needs {}, which is missing",
            helper.display()
        )));
    }
    for value in entries(helper, icon, inbox, &tags::catalog(db)?) {
        win32::set_user_string(&value.key, value.name.as_deref(), &value.value)?;
    }
    Ok(())
}

/// Every registry string the menu is made of, for both roots: the submenu, then one
/// entry per tag in catalog order (the key names sort that way), each running
/// `"<helper>" --inbox "<inbox>" --tag <tag> "%V"`. A tag is letters, digits, `-`
/// and `_` only, so it needs no quoting; `%V` is the folder Explorer passes.
pub fn entries(helper: &Path, icon: &Path, inbox: &Path, catalog: &[TagInfo]) -> Vec<RegValue> {
    let value = |key: &str, name: Option<&str>, value: String| RegValue {
        key: key.to_owned(),
        name: name.map(str::to_owned),
        value,
    };
    let mut values = Vec::new();
    for root in ROOTS {
        values.push(value(root, Some("MUIVerb"), "Tag with Findex".into()));
        values.push(value(root, Some("Icon"), format!("{},0", icon.display())));
        values.push(value(root, Some("SubCommands"), String::new()));
        for (index, info) in catalog.iter().take(MAX_ITEMS).enumerate() {
            let item = format!(r"{root}\shell\{:02}", index + 1);
            values.push(value(&item, Some("MUIVerb"), label(info)));
            values.push(value(
                &format!(r"{item}\command"),
                None,
                format!(
                    r#""{}" --inbox "{}" --tag {} "%V""#,
                    helper.display(),
                    inbox.display(),
                    info.tag
                ),
            ));
        }
    }
    values
}

/// A preset as "Working", a custom tag as "#fw", as the window writes them.
fn label(info: &TagInfo) -> String {
    if info.preset {
        let mut chars = info.tag.chars();
        chars
            .next()
            .map(|first| first.to_uppercase().chain(chars).collect())
            .unwrap_or_default()
    } else {
        format!("#{}", info.tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tag(name: &str, preset: bool) -> TagInfo {
        TagInfo {
            tag: name.into(),
            color: None,
            preset,
            folders: 0,
        }
    }

    #[test]
    fn each_tag_runs_the_helper_on_the_folder_explorer_passes() {
        let helper = PathBuf::from(r"C:\Program Files\Findex\findex-tag.exe");
        let icon = PathBuf::from(r"C:\Program Files\Findex\findex.exe");
        let inbox = PathBuf::from(r"C:\Users\a b\AppData\Roaming\com.dtbao.findex\inbox");
        let values = entries(
            &helper,
            &icon,
            &inbox,
            &[tag("working", true), tag("fw", false)],
        );

        let folder: Vec<_> = values
            .iter()
            .filter(|v| v.key.starts_with(ROOTS[0]))
            .collect();
        assert_eq!(
            folder
                .iter()
                .map(|v| (v.key.as_str(), v.name.as_deref(), v.value.as_str()))
                .collect::<Vec<_>>(),
            [
                (ROOTS[0], Some("MUIVerb"), "Tag with Findex"),
                (
                    ROOTS[0],
                    Some("Icon"),
                    r"C:\Program Files\Findex\findex.exe,0"
                ),
                (ROOTS[0], Some("SubCommands"), ""),
                (
                    r"Software\Classes\Directory\shell\FindexTag\shell\01",
                    Some("MUIVerb"),
                    "Working"
                ),
                (
                    r"Software\Classes\Directory\shell\FindexTag\shell\01\command",
                    None,
                    r#""C:\Program Files\Findex\findex-tag.exe" --inbox "C:\Users\a b\AppData\Roaming\com.dtbao.findex\inbox" --tag working "%V""#
                ),
                (
                    r"Software\Classes\Directory\shell\FindexTag\shell\02",
                    Some("MUIVerb"),
                    "#fw"
                ),
                (
                    r"Software\Classes\Directory\shell\FindexTag\shell\02\command",
                    None,
                    r#""C:\Program Files\Findex\findex-tag.exe" --inbox "C:\Users\a b\AppData\Roaming\com.dtbao.findex\inbox" --tag fw "%V""#
                ),
            ]
        );
        // The background of an open folder carries the same menu.
        assert_eq!(values.len(), folder.len() * 2);
        assert!(values
            .iter()
            .any(|v| v.key == r"Software\Classes\Directory\Background\shell\FindexTag\shell\02"));
    }

    #[test]
    fn the_submenu_stops_at_sixteen_tags() {
        let many: Vec<_> = (0..20).map(|i| tag(&format!("t{i}"), false)).collect();
        let values = entries(
            Path::new("h.exe"),
            Path::new("f.exe"),
            Path::new("in"),
            &many,
        );
        let commands = values
            .iter()
            .filter(|v| v.key.ends_with(r"\command"))
            .count();
        assert_eq!(commands, 2 * 16);
    }

    #[test]
    fn turned_off_means_no_menu() {
        let db = crate::model::db::open_in_memory().unwrap();
        assert!(enabled(&db).unwrap());
        db.set_setting(SETTING, "off").unwrap();
        assert!(!enabled(&db).unwrap());
        db.set_setting(SETTING, "on").unwrap();
        assert!(enabled(&db).unwrap());
    }

    #[test]
    fn registry_strings_round_trip_and_trees_delete() {
        let root = format!(r"Software\findex-test-{}", std::process::id());
        let key = format!(r"{root}\a\b");
        win32::set_user_string(&key, None, r#""C:\x y\z.exe" "%V""#).unwrap();
        win32::set_user_string(&key, Some("MUIVerb"), "Working").unwrap();
        assert_eq!(
            win32::get_user_string(&key, None).as_deref(),
            Some(r#""C:\x y\z.exe" "%V""#)
        );
        assert_eq!(
            win32::get_user_string(&key, Some("MUIVerb")).as_deref(),
            Some("Working")
        );
        win32::delete_user_tree(&root).unwrap();
        assert_eq!(win32::get_user_string(&key, None), None);
        // Deleting what is not there is not an error.
        win32::delete_user_tree(&root).unwrap();
    }
}
