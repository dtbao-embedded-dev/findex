import { useEffect, useState } from "react";

import { needsSetup } from "../presenter/setupPresenter";
import { revealMain } from "../presenter/splashPresenter";
import { summarize, watchStatus } from "../presenter/statusPresenter";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "../model/window";
import { SearchView } from "./SearchView";
import { SetupView } from "./SetupView";
import { StatusBar } from "./StatusBar";
import { TitleBar } from "./TitleBar";

type Screen = "loading" | "setup" | "search";

/**
 * The root component. It holds view state and hands user input to the presenter;
 * deciding what that input means is the presenter's job, not the component's.
 */
export function App() {
  const [screen, setScreen] = useState<Screen>("loading");
  const [firstRun, setFirstRun] = useState(false);
  const [status, setStatus] = useState(() => summarize({ drives: [] }));

  useEffect(() => {
    const route = needsSetup().then((setup) => {
      setFirstRun(setup);
      setScreen(setup ? "setup" : "search");
    });
    void revealMain(route);
    const stop = watchStatus(setStatus);
    return () => {
      void stop.then((unlisten) => unlisten());
    };
  }, []);

  return (
    <div className="app">
      <TitleBar
        title="Findex"
        onMinimize={minimizeWindow}
        onToggleMaximize={toggleMaximizeWindow}
        onClose={closeWindow}
      />
      {screen === "setup" && (
        <div className="screen-scroll">
          <SetupView
            firstRun={firstRun}
            onDone={() => {
              setFirstRun(false);
              setScreen("search");
            }}
            onCancel={() => setScreen("search")}
          />
        </div>
      )}
      {screen === "search" && (
        <SearchView total={status.total} onOpenDrives={() => setScreen("setup")} />
      )}
      {screen !== "loading" && <StatusBar line={status} />}
    </div>
  );
}
