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
- The app opens straight on search and indexes one drive at a time, the system drive
  on the first launch. A drive button beside the search field lists every local drive
  with its label, free space and filesystem, and whether it is indexed instantly (NTFS)
  or by a folder scan. Picking one indexes that drive alone; the drive it replaces
  keeps its saved index, so coming back to it is quick. Network and optical drives are
  not offered: a disconnected network drive can hang the list.
- The active drive is indexed in the background as soon as the app starts, and again
  whenever another drive is picked:
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
- Right-clicking a result opens its actions; the menu key or Shift+F10 opens them for
  the selected result, and Esc closes them.
  - **Folders:** Open with Claude (Claude Code, `claude`, in a terminal at the folder),
    Open folder (File Explorer), Open in Terminal (Windows Terminal, or PowerShell
    without it), Copy path and Copy name.
  - **Files:** Show in Explorer, Copy path and Copy name.
  - **Rights:** terminals and Claude start with your normal rights, not the
    administrator rights Findex runs with, so a tool opened from a search cannot
    change the system by surprise. If that is not possible, they are not started.
  - **Folder names:** the folder becomes the terminal's working directory instead of
    an argument, so a name with spaces or `;` opens the right place.
  - **Untrusted folders:** Claude Code is started by the full path found on PATH, and
    programs are not looked up in the folder, so a `claude.cmd` inside a cloned repo
    cannot run in its place. If Findex cannot tell whether it is elevated, it refuses
    to start them rather than risk administrator rights.
  - A copy is confirmed by a short note above the status bar; a failure (for example
    `claude` not found on PATH) is shown there in red.
- Each result shows what it is, how big it is and when it last changed, at the right
  of its row: Explorer's name for the type (for example "PDF Document" or "File
  folder"), the size for files, and the last-modified date in your system's format.
  They are read from the disk after the results appear, so searching is as fast as
  before; a page of 200 results takes a few milliseconds more to fill in.
- Folders can be tagged. Right-click a folder and choose Tags… to add a tag (Enter) or
  remove one; tags already in use are offered with one press.
  - **Finding them:** `tag:work` lists the folders tagged `work`, on any drive, and
    combines with name words (`proj tag:work`) and several tags (`tag:work;fw`).
  - **Where they show:** as chips beside a folder's name in the results, and in the
    sidebar with how many folders carry each; pressing one filters by it.
  - **What a tag is:** one word of up to 32 letters, digits, `-` or `_`, stored in
    lower case. Tags are saved in the app's database, keyed by the folder's path, so a
    folder that is renamed or moved loses its tags.
- Tags come in two kinds, as in Finder:
  - **Colour tags:** Red, Orange, Yellow, Green, Blue, Purple and Gray are always
    there, drawn as coloured dots in the sidebar, the filters, the tag editor and on
    results, and cannot be deleted. The tag editor offers them as a row of dots to
    toggle.
  - **Your own tags:** made in Settings → Tags, or by typing a new name in the tag
    editor. Settings lists them with their folder counts; deleting one asks first and
    takes it off every folder. A tag that already exists is refused rather than
    merged.
  - **Upgrading:** tags already on folders become your own tags; a folder tagged
    `red` before keeps it, now as the Red colour tag.
- A filter bar under the search field narrows the results. Every change searches
  again at once.
  - **Date:** when a result last changed: Today, Yesterday, Last 7 days, Last 30
    days, This month, This year, or a custom day, month, year or range (either end
    may be left open). Days follow your local time.
  - **Kind:** All, Files or Folders, overriding a typed `type:`.
  - **Tags:** folders carrying every tag picked, from the filter's menu or the
    sidebar; a tag chip on a result turns its filter on or off.
  - **How dates are read:** the index holds names only, so dates are read from the
    disk for the best name matches, up to 20,000 to 100,000 of them depending on the
    results limit. When that was not enough, the line above the results says to type
    more of the name. A one-letter search checked against the last day took under a
    second on 1.2 million names.
  - **Without a name:** a date or kind alone lists nothing, since there is no name to
    look for; a tag alone lists the folders carrying it.
  - **Esc:** the first Esc clears the name, a second one clears the filters.
  - **While files change:** a search with a date filter is not re-run on its own as
    files change, since each run reads thousands of dates from the disk; typing or
    changing a filter runs it again. Custom dates before 1970 are not accepted.
- Settings (the gear in the title bar) open as a sheet over the window. The first
  setting is **Results per search**, from 200 (the default) to 10,000, typed, slid or
  picked from 200 / 1,000 / 5,000 / 10,000. It applies to the search on screen at
  once and is kept across restarts.
  - **Checked twice:** a value out of range is refused with a note and the saved one
    kept; Rust checks it again, and falls back to 200 if a stored value is ever
    out of range.
  - **Cost:** more results take longer. In a development build a one-letter search
    on 1.2 million names took about 0.4 s for 10,000 results against 0.07 s for 200.
- The result list renders only the rows in view and reads type, size and date only
  for them, so 10,000 results scroll as smoothly as 200. ↑ from the first result
  still jumps to the last, and the menu key opens its actions there.
- The line above the results shows how much memory the drive's name index holds,
  beside the search time: "Top 200 results · 18 ms · 133 MB RAM". It counts what the
  index's containers hold, so it is lower than the app's working set in Task Manager,
  which also holds the window and the rest of the app (133 MB against 191 MB for
  1.2 million names here).
- A re-scan button (the circular arrow in the title bar) reads the active drive again
  from scratch, for when its index looks out of date. An NTFS drive is read from its
  file table again instead of being caught up from its journal. The previous results
  stay searchable until the new index replaces them; the button turns and waits
  while any indexing runs, and the status bar shows the progress.
- Findex updates itself. It checks for a newer signed release 10 seconds after it
  opens and every six hours, downloads it in the background, then offers "Restart
  now" or "Later" in a card above the status bar. "Later" keeps the download and
  offers it again at the next check.
  - **From the tray:** "Check for updates" checks at once and answers either way:
    "Findex is up to date", or why the check failed.
  - **Quiet by default:** a background check that finds nothing, or cannot reach the
    server (offline, or no release published yet), shows nothing.
  - **Before installing:** the drive index changed by live updates is saved first,
    since the installer closes the app without the usual quit.

### Changed

- The main window is redesigned after Finder: a sidebar on the left holds the drive
  (with its free space) and the tags in use with their folder counts, and the search
  sits on the right. Pressing a tag in the sidebar filters by it, in place of the
  chips the empty screen showed before.
  - **Results:** one fixed-height row per result, and the selected row is filled with
    the accent colour so it is found at a glance.
  - **Chrome:** title bar, sidebar and status bar share one translucent material, and
    menus, the tag editor and the update card float on a solid surface above them.
  - **Dark mode, reduced motion, reduced transparency and more contrast** are all
    followed, and the splash window picks up the same look.
  - **Corners:** every panel, field, button, chip and row has the same 5 px corner
    radius; only the colour dots and the status dot stay round.
- The title bar shows the app icon, the name and the running version (for example
  `Findex v0.1.0`), so which build is open is visible without the tray.
- The app is now named **Findex** in its window, title bar, tray tooltip, installer and
  release titles. The package, crate, bundle id (`com.dtbao.findex`) and repository stay
  lowercase, so installed copies and update URLs are unaffected.
- A real app icon (folder with a magnifier) replaces the placeholder; its source is
  `src-tauri/icons/icon.svg` and the README now shows it.
- The app now asks for administrator rights (UAC) whenever it opens, because reading
  an NTFS file table requires them.
  - **Installed app:** asks through its manifest.
  - **Build started any other way:** relaunches itself through UAC.
  - **If you say No:** it still opens and indexes by folder scan.
  - **`pnpm tauri dev`:** never relaunched, since that would stop the dev server. Run
    the terminal as administrator instead.
- Right-clicking no longer opens the browser's own menu (Back, Reload, Inspect…), which
  belongs to a web page rather than a desktop app. Text fields keep it, for cut, copy
  and paste.
- The status bar explains a folder scan in plain words. When rights are missing it
  says "open Findex as administrator for instant search and live updates" instead of
  "Access is denied (os error 5)".

### Fixed

### Removed

- The scaffold's greeting demo (`greet` command and its view), replaced by search.
