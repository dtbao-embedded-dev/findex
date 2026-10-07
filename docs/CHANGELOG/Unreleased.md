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
  - **Keyboard:** ↑/↓ move the selection and Esc clears the field. Enter and a double
    click open nothing, so a stray key press never launches a window; every action on
    a result is in its right-click menu.
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
  - **Preset tags:** Working, Doc, Important, Todo, Review, Personal and Archive are
    always there, each drawn with its own coloured dot beside its name in the sidebar,
    the filters, the tag editor and on results, and cannot be deleted. They are named
    for what a folder is, not for a colour, so a tag says why it was set. The tag editor
    offers them as a row of pills to toggle.
  - **Your own tags:** made in Settings → Tags, or by typing a new name in the tag
    editor. Settings lists them with their folder counts; deleting one asks first and
    takes it off every folder. A tag that already exists is refused rather than
    merged.
  - **Upgrading:** tags already on folders become your own tags. A folder on one of
    the earlier colour tags moves to the preset of the same colour (Blue → Working,
    Green → Doc, Red → Important, Orange → Todo, Yellow → Review, Purple → Personal,
    Gray → Archive); one of your own tags already named like a preset becomes that
    preset, keeping its folders.
- A filter bar under the search field narrows the results. Every change searches
  again at once.
  - **Date:** when a result last changed: Today, Yesterday, Last 7 days, Last 30
    days, This month, This year, or a custom day, month, year or range (either end
    may be left open). Days follow your local time.
  - **Kind:** All, Files or Folders, overriding a typed `type:`.
  - **Type:** file types to keep. Pick a group (Code, Documents, Images, Archives) or
    type extensions such as `.c, .h`; a typed `ext:` still works and adds to them.
    Searching `code` with `.c` lists only C files whose name holds "code"; a type
    alone lists every file of that type.
  - **Tags:** folders carrying every tag picked, from the filter's menu or the
    sidebar; a tag chip on a result turns its filter on or off.
  - **How dates are read:** the index holds names only, so dates are read from the
    disk for the best name matches, 20,000 to 100,000 of them depending on how far
    the list was scrolled. When that was not enough, the line above the results says
    to type more of the name. A one-letter search checked against the last day took under a
    second on 1.2 million names.
  - **Without a name:** a date or kind alone lists nothing, since there is no name to
    look for; a tag alone lists the folders carrying it, and a type alone every file of
    that type.
  - **Esc:** the first Esc clears the name, a second one clears the filters.
  - **While files change:** a search with a date filter is not re-run on its own as
    files change, since each run reads thousands of dates from the disk; typing or
    changing a filter runs it again. Custom dates before 1970 are not accepted.
- Settings (the gear in the title bar) open as a sheet over the window, holding the
  tags.
- A search has no cap on results. The best 500 show at once, and the next 500 load
  as the list scrolls near the end, until every match is listed. The line above the
  results counts every match, not only the rows loaded ("48,213 results"); with a
  date filter, where the count is only known once every date was read, it says
  "500+ results" until then. Each page ranks the drive again, so pages follow one
  order; a file that appears between two pages is never listed twice.
- The result list renders only the rows in view and reads type, size and date only
  for them, so a list of a hundred thousand rows scrolls as smoothly as one of 200.
  ↑ from the first result still jumps to the last loaded, and the menu key opens its
  actions there.
- The line above the results shows how much memory the drive's name index holds,
  beside the search time: "48,213 results · 18 ms · 133 MB RAM". It counts what the
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
- Search inside files. **Name | Contents** at the start of the filter bar picks
  where the words are looked for, as in Finder. A content search lists the files
  whose name matches first, ranked, then every file holding all the typed words, each
  with the line number and the text of the first matching line, the words in bold
  (the folder shows on hover). Results arrive while the drive is still being read:
  the line above them says "Searching contents… 37 results · 1,204 of 52,310 files
  read", then "37 results · 52,310 files read · 4.2 s".
  - **Typing:** the search starts once typing pauses for a quarter of a second, and
    a new search stops the old one; its results never come back. Going back to Name
    or clearing the field stops it too, so the drive is not read on in the
    background. A content search is not re-run on its own as files change.
  - **Progress:** files skipped (too large, binary, locked, outside the date range)
    count as looked at, so the count reaches the total.
  - **Measured on E: (development build, unelevated):** 49,585 C files read in 37 s
    the first time and 3.3 s once the disk cache held them; the first of 617 matches
    showed after a second.
  - **What is read:** files of a text type (code, text, markup, configuration, logs,
    CSV, KiCad design files and the like), or only the types picked in the Type
    filter. Files over 8 MB and files that look binary are skipped. Matching ignores
    case in any script ("ảnh" finds "Ảnh").
  - **Filters:** a tag keeps the files inside the folders carrying it; a date keeps
    the files changed in that range.
  - **No index:** nothing is stored, so a search always reads the files as they are
    now. Reading a whole drive takes as long as the disk needs; PDF and Word files are
    not text, so their contents are not found.
  - **Rights:** a release build runs as administrator, so it can read, and list
    lines from, files your own account cannot open.
- **Tag with Findex** in Explorer's right-click menu, on a folder and on the empty
  space of an open folder: a submenu with one entry per tag (presets first, up to 16)
  tags that folder while Findex stays in the tray. On Windows 11 it is under "Show
  more options".
  - **How:** each entry runs `findex-tag.exe`, a small helper installed beside Findex
    that runs with your normal rights (Findex itself runs as administrator), and leaves
    a request in Findex's `inbox` folder. Findex applies it within a second and the
    tag shows on the result at once; if Findex is not running, at its next start.
  - **Settings → Explorer** turns the menu on or off (on by default) and says why if
    it cannot be set up. The menu follows the tags: a new or deleted tag shows there
    the next time it is opened. Uninstalling Findex removes the menu.
- Folder results show their size beside their type ("File folder · 2.4 GB"). It is
  measured in the background for the folders on screen, reads "Calculating…" until
  known, and is remembered for five minutes. Junctions and links are not followed and
  unreadable subfolders count as empty; a folder that cannot be read shows no size.
  Measuring uses four threads of its own, so a large folder never slows the search,
  and scrolling past a folder stops measuring it.
- **Open with VS Code** in the right-click menu of both folders and files. A folder
  opens as its own VS Code window, a file in VS Code's current window. Code is found
  beside the `code` command on PATH, else in the default per-user or machine-wide
  install, and starts with your normal rights, like the terminal.
- **Recent searches** drop down from the search field when it is empty and has
  focus. Pick one with a click, or with ↑/↓ and Enter; Esc puts the list away, the ×
  on a row forgets that search, and **Clear** forgets them all. A search is remembered
  once it has stayed on screen with results for 1.5 s, or when a result's menu was
  used; typing on from the newest entry ("rea", then "readme") replaces it rather
  than adding both. The last 20 are kept across restarts.
- A round **back to the top** button floats over the lower right of the results once
  they are scrolled past one screen; it scrolls smoothly back up (instantly with
  reduced motion) and then hides.

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
- The sidebar is gone; the search now fills the window. The drive picker is a
  compact button (drive glyph and letter, e.g. `E:`) at the left of the search field,
  with the drive's name and free space on hover, and the re-scan button moved from the
  title bar to the right of the field. Tags are still filtered from the **Tags** menu
  in the filter bar and from the tag chips on results.
- Findex is now **dark only**, whatever Windows' light or dark setting, and
  **Settings → Appearance** offers six themes: Graphite (the dark look so far, and
  the default), Midnight, Aurora, Violet, Forest and Ember. Each swatch previews its
  theme; picking one applies it at once, is remembered, and is worn by the splash
  window too. On the lighter accents (Aurora, Forest, Ember) the selected row's text
  turns near black so it stays readable.
- Building now also builds `findex-tag`, a small helper crate in `src-tauri/tag-helper`
  (a Cargo workspace member). `pnpm tauri dev` builds it in debug beside the app;
  `pnpm tauri build` runs `docs/scripts/build-helper.py` (Python), which builds it in
  release and places it where the release bundle (`bundle.externalBin` in
  `tauri.release.conf.json`) picks it up. CI now formats, lints and tests the whole
  workspace.

### Fixed

- In dark mode the chosen segment of a segmented control (Name | Contents, All |
  Files | Folders) was darker than its track and hard to see; it is now lifted in a
  lighter grey, as macOS draws it.

### Removed

- The scaffold's greeting demo (`greet` command and its view), replaced by search.
