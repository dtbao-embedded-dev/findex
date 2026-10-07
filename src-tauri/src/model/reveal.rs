//! Shows a search result in File Explorer, selected in its folder.

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::error::{AppError, AppResult};

/// Opens Explorer on the folder holding `path`, with `path` selected. A result that
/// was deleted since it was indexed is refused rather than opening the wrong place.
pub fn reveal(path: &str) -> AppResult<()> {
    if !Path::new(path).exists() {
        return Err(AppError::Invalid(format!("{path} no longer exists")));
    }
    // Explorer parses its own command line: the path goes in quotes after the comma,
    // passed raw so Rust's argument quoting cannot wrap the whole switch. Windows
    // paths cannot contain `"`, so the quotes cannot be broken out of.
    Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{path}\""))
        .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_a_path_that_no_longer_exists() {
        let missing = std::env::temp_dir().join("findex-reveal-missing-file.txt");
        let result = reveal(&missing.to_string_lossy());
        assert!(matches!(result, Err(AppError::Invalid(_))));
    }
}
