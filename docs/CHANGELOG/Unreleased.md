# Unreleased

> Write every change here as it lands, so the release does not have to be
> reconstructed from git history later. `docs/scripts/tool-release.py` renames this
> file to `<tag>.md` at release time and leaves a fresh copy of this skeleton behind.

### Added

- Release builds are signed for the updater: the real public key is embedded, so the
  first published version can receive later updates without a manual reinstall.
- A file and folder name search engine. It keeps each drive's names in one compact
  in-memory index and searches it on every core at once. Every typed word must appear
  in the name; `ext:pdf;docx` and `type:file` / `type:folder` narrow the results.
  Matches rank as exact name, then prefix, then word start, then anywhere. A million
  names answer in about 15 ms, so results can follow every keystroke.
- A splash window shows the icon, version and copyright while the main window loads.
  It stays up for at least 0.9 s so it never just flashes. If the main window fails to
  report ready, it is shown anyway after 10 s. The copyright is read from
  `bundle.copyright` in `tauri.conf.json`, the same value the installer uses.

### Changed

- The app is now named **Findex** in its window, title bar, tray tooltip, installer and
  release titles. The package, crate, bundle id (`com.dtbao.findex`) and repository stay
  lowercase, so installed copies and update URLs are unaffected.
- A real app icon (folder with a magnifier) replaces the placeholder; its source is
  `src-tauri/icons/icon.svg` and the README now shows it.

### Fixed

### Removed
