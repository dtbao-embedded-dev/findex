# Unreleased

> Write every change here as it lands, so the release does not have to be
> reconstructed from git history later. `docs/scripts/tool-release.py` renames this
> file to `<tag>.md` at release time and leaves a fresh copy of this skeleton behind.

### Added

- Release builds are signed for the updater: the real public key is embedded, so the
  first published version can receive later updates without a manual reinstall.

### Changed

- The app is now named **Findex** in its window, title bar, tray tooltip, installer and
  release titles. The package, crate, bundle id (`com.dtbao.findex`) and repository stay
  lowercase, so installed copies and update URLs are unaffected.
- A real app icon (folder with a magnifier) replaces the placeholder; its source is
  `src-tauri/icons/icon.svg` and the README now shows it.

### Fixed

### Removed
