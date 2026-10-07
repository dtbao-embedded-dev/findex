//! What the splash window says about the app: name, version and copyright, all read
//! from the bundle config so a release bump never needs a code change.

use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct About {
    pub name: String,
    pub version: String,
    pub copyright: String,
}

/// Collects the about text; a missing copyright shows as nothing rather than failing.
pub fn about(name: &str, version: &str, copyright: Option<&str>) -> About {
    About {
        name: name.to_owned(),
        version: version.to_owned(),
        copyright: copyright.unwrap_or_default().trim().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_the_bundle_values() {
        let about = about("Findex", "0.1.0", Some(" © 2026 dtbao "));
        assert_eq!(
            about,
            About {
                name: "Findex".into(),
                version: "0.1.0".into(),
                copyright: "© 2026 dtbao".into(),
            }
        );
    }

    #[test]
    fn a_missing_copyright_is_empty() {
        assert_eq!(about("Findex", "0.1.0", None).copyright, "");
    }
}
