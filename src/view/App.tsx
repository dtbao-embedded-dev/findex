import { useCallback, useEffect, useMemo, useState } from "react";

import { initialTitle, loadTitle } from "../presenter/aboutPresenter";
import { loadActive } from "../presenter/drivePresenter";
import type { TagInfo } from "../model/tags";
import { loadLimit, RESULTS_DEFAULT } from "../presenter/settingsPresenter";
import { revealMain } from "../presenter/splashPresenter";
import { loadTags } from "../presenter/tagPresenter";
import { summarize, watchStatus } from "../presenter/statusPresenter";
import {
  createUpdater,
  listenForChecks,
  scheduleChecks,
  type UpdateState,
} from "../presenter/updatePresenter";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "../model/window";
import { SearchView } from "./SearchView";
import { SettingsSheet } from "./SettingsSheet";
import { TagSettings } from "./TagSettings";
import { StatusBar } from "./StatusBar";
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
  const [limit, setLimit] = useState(RESULTS_DEFAULT);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const closeSettings = useCallback(() => setSettingsOpen(false), []);
  // The tag catalog, shared by the sidebar, the filters, the tag editor and the settings.
  const [tags, setTags] = useState<TagInfo[]>([]);
  const refreshTags = useCallback(() => void loadTags().then(setTags), []);
  const [status, setStatus] = useState(() => summarize({ drives: [] }));
  const [update, setUpdate] = useState<UpdateState>({ kind: "idle" });
  const updater = useMemo(() => createUpdater({ onState: setUpdate }), []);

  useEffect(() => {
    void loadTitle().then(setTitle);
    void loadLimit().then(setLimit);
    refreshTags();
    void revealMain(loadActive().then(setActive));
    const stop = watchStatus(setStatus);
    return () => {
      void stop.then((unlisten) => unlisten());
    };
  }, []);

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
      />
      <SearchView
        total={status.total}
        active={active}
        onActiveChange={setActive}
        limit={limit}
        tags={tags}
        onTagsChanged={refreshTags}
      />
      <UpdateBanner
        state={update}
        onRestart={() => void updater.restart()}
        onLater={updater.later}
      />
      <StatusBar line={status} />
      {settingsOpen && (
        <SettingsSheet limit={limit} onLimitChange={setLimit} onClose={closeSettings}>
          <TagSettings catalog={tags} onCatalog={setTags} />
        </SettingsSheet>
      )}
    </div>
  );
}
