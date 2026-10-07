import { useEffect, useMemo, useState } from "react";

import { initialTitle, loadTitle } from "../presenter/aboutPresenter";
import { loadActive } from "../presenter/drivePresenter";
import { revealMain } from "../presenter/splashPresenter";
import { summarize, watchStatus } from "../presenter/statusPresenter";
import {
  createUpdater,
  listenForChecks,
  scheduleChecks,
  type UpdateState,
} from "../presenter/updatePresenter";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "../model/window";
import { SearchView } from "./SearchView";
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
  const [status, setStatus] = useState(() => summarize({ drives: [] }));
  const [update, setUpdate] = useState<UpdateState>({ kind: "idle" });
  const updater = useMemo(() => createUpdater({ onState: setUpdate }), []);

  useEffect(() => {
    void loadTitle().then(setTitle);
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
      />
      <SearchView total={status.total} active={active} onActiveChange={setActive} />
      <UpdateBanner
        state={update}
        onRestart={() => void updater.restart()}
        onLater={updater.later}
      />
      <StatusBar line={status} />
    </div>
  );
}
