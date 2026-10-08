import { useCallback, useEffect, useMemo, useState } from "react";

import { initialTitle, loadTitle } from "../presenter/aboutPresenter";
import { menuTagsKey, syncMenu } from "../presenter/explorerPresenter";
import { DEFAULT_THEME, loadTheme, saveTheme, type ThemeId } from "../presenter/themePresenter";
import { loadActive } from "../presenter/drivePresenter";
import type { TagInfo } from "../model/tags";
import { revealMain } from "../presenter/splashPresenter";
import { loadTags, watchTags } from "../presenter/tagPresenter";
import { rescan, rescanLabel, summarize, watchStatus } from "../presenter/statusPresenter";
import {
  createUpdater,
  listenForChecks,
  scheduleChecks,
  type UpdateState,
} from "../presenter/updatePresenter";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "../model/window";
import { AppearanceSettings } from "./AppearanceSettings";
import { ExplorerSettings } from "./ExplorerSettings";
import { SearchView } from "./SearchView";
import { SettingsSheet } from "./SettingsSheet";
import { TagSettings } from "./TagSettings";
import { StatusBar } from "./StatusBar";
import { Toast } from "./Toast";
import { TitleBar } from "./TitleBar";
import { UpdateBanner } from "./UpdateBanner";

/**
 * The root component. It holds view state and hands user input to the presenter;
 * deciding what that input means is the presenter's job, not the component's.
 * The app opens straight on search: Rust has already picked the drive to index.
 * Updates are checked in the background; a ready one is offered above the status bar.
 */
export function App() {
  const [title, setTitle] = useState(initialTitle);
  const [active, setActive] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  // The colour theme, set on <html> so every surface and the settings follow it.
  const [theme, setTheme] = useState<ThemeId>(DEFAULT_THEME);
  // Why a re-scan could not start; it fades like any other note.
  const [rescanError, setRescanError] = useState<string | null>(null);
  const closeSettings = useCallback(() => setSettingsOpen(false), []);
  // The tag catalog, shared by the filters, the tag editor and the settings.
  const [tags, setTags] = useState<TagInfo[]>([]);
  const refreshTags = useCallback(() => void loadTags().then(setTags), []);
  // Explorer's "Tag with Findex" menu lists the tags, so it is rewritten when their
  // names change; a failure shows when the settings are opened.
  const tagsKey = menuTagsKey(tags);
  useEffect(() => {
    if (tagsKey !== "") {
      void syncMenu();
    }
  }, [tagsKey]);
  const [status, setStatus] = useState(() => summarize({ drives: [] }));
  const [update, setUpdate] = useState<UpdateState>({ kind: "idle" });
  const updater = useMemo(() => createUpdater({ onState: setUpdate }), []);

  useEffect(() => {
    void loadTitle().then(setTitle);
    refreshTags();
    // The window is shown once the drive and the theme are known, so it never
    // appears in the wrong colours first.
    void revealMain(Promise.all([loadActive().then(setActive), loadTheme().then(setTheme)]));
    const stop = watchStatus(setStatus);
    // A folder tagged from Explorer's menu shows its tag here without a restart.
    const stopTags = watchTags(setTags);
    return () => {
      void stop.then((unlisten) => unlisten());
      void stopTags.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    if (rescanError === null) {
      return;
    }
    const timer = setTimeout(() => setRescanError(null), 4000);
    return () => clearTimeout(timer);
  }, [rescanError]);

  useEffect(() => {
    const stopChecks = scheduleChecks(() => void updater.check(false));
    const stopTray = listenForChecks(() => void updater.check(true));
    return () => {
      stopChecks();
      void stopTray.then((unlisten) => unlisten());
    };
  }, [updater]);

  return (
    <div className="app">
      <TitleBar
        title={title}
        onMinimize={minimizeWindow}
        onToggleMaximize={toggleMaximizeWindow}
        onClose={closeWindow}
        onSettings={() => setSettingsOpen(true)}
        update={update}
        onUpdate={() => void updater.check(true)}
      />
      <SearchView
        total={status.total}
        active={active}
        onActiveChange={setActive}
        tags={tags}
        onTagsChanged={refreshTags}
        onRescan={() => void rescan().then(setRescanError)}
        rescanLabel={rescanLabel(active, status.busy)}
        busy={status.busy}
      />
      <UpdateBanner
        state={update}
        onRestart={() => void updater.restart()}
        onLater={updater.later}
      />
      <StatusBar line={status} copyright={title.copyright} />
      {rescanError !== null && <Toast text={rescanError} tone="error" />}
      {settingsOpen && (
        <SettingsSheet onClose={closeSettings}>
          <AppearanceSettings
            theme={theme}
            onTheme={(next) => {
              setTheme(next);
              void saveTheme(next);
            }}
          />
          <TagSettings catalog={tags} onCatalog={setTags} />
          <ExplorerSettings />
        </SettingsSheet>
      )}
    </div>
  );
}
