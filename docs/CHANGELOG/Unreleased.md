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
- The first launch asks which drives to index. Each local drive is shown with its
  label, free space and filesystem, and C: is ticked by default. NTFS drives are marked
  as indexed instantly and kept current; other filesystems get a folder scan. The
  choice is saved, and the Drives button reopens the screen to change it. Network and
  optical drives are not offered: a disconnected network drive can hang the list.
- Chosen drives are indexed in the background as soon as the app starts, and again
  whenever the choice changes:
  - **NTFS drives:** read straight from the drive's file table, which takes seconds for
    millions of names.
  - **Other drives:** a parallel folder scan that never follows junctions or links.
  - **While re-indexing:** a drive's previous results stay searchable until its new
    index replaces them.
- The main window is a search screen:
  - **Search as you type:** results appear on every keystroke, with the matched part
    of each name in bold and its folder underneath. An answer that arrives late is
    never allowed to replace a newer one.
  - **Keyboard:** ↑/↓ move the selection, Enter (or a double click) shows the file in
    Explorer, and Esc clears the field.
  - **Status bar:** shows indexing progress per drive, then how many items can be
    searched, plus why a drive fell back to a folder scan.
  - **Measured:** on 6.3 million names in a development build, a typical query
    answers in about 30 ms, round trip included.
- Each drive's index is saved to `%APPDATA%\com.dtbao.findex\index\<drive>.fxi` after
  it is built.
  - **At the next launch:** the saved index is loaded first, so search works within
    about a second while the drive is indexed again behind it. A million names load in
    about 0.2 s.
  - **Damaged files:** a damaged or outdated file is detected by its checksum and
    version, then ignored and rebuilt.
- NTFS drives stay current while the app runs: created, renamed, moved and deleted
  files show up in results within about a second, read from the drive's change
  journal.
  - **Next launch:** the saved index catches up from the journal instead of being
    rebuilt.
  - **Journal gaps:** if the journal no longer reaches back that far, the drive is
    rebuilt once and followed again.
  - **When the index is saved:** every ten minutes while it changes, and when the app
    quits.
- A search typed before indexing finished now refreshes on its own when more of the
  drive becomes searchable, instead of staying on "No matches". A background refresh
  keeps the selected file selected.
- Searches no longer stall while a drive is being folder-scanned. The scan now runs on
  its own threads, so it no longer competes with search for the shared pool.

### Changed

- The app is now named **Findex** in its window, title bar, tray tooltip, installer and
  release titles. The package, crate, bundle id (`com.dtbao.findex`) and repository stay
  lowercase, so installed copies and update URLs are unaffected.
- A real app icon (folder with a magnifier) replaces the placeholder; its source is
  `src-tauri/icons/icon.svg` and the README now shows it.
- The installed app now asks for administrator rights (UAC) when it starts. Reading
  an NTFS file table requires them. Development builds still run unelevated and use
  the folder scan.

### Fixed

### Removed

- The scaffold's greeting demo (`greet` command and its view), replaced by search.
