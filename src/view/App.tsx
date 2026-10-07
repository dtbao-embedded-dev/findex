import { useEffect, useState } from "react";

import { loadActive } from "../presenter/drivePresenter";
import { revealMain } from "../presenter/splashPresenter";
import { summarize, watchStatus } from "../presenter/statusPresenter";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "../model/window";
import { SearchView } from "./SearchView";
import { StatusBar } from "./StatusBar";
import { TitleBar } from "./TitleBar";

/**
 * The root component. It holds view state and hands user input to the presenter;
 * deciding what that input means is the presenter's job, not the component's.
 * The app opens straight on search: Rust has already picked the drive to index.
 */
export function App() {
  const [active, setActive] = useState<string | null>(null);
  const [status, setStatus] = useState(() => summarize({ drives: [] }));

  useEffect(() => {
    void revealMain(loadActive().then(setActive));
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
      <SearchView total={status.total} active={active} onActiveChange={setActive} />
      <StatusBar line={status} />
    </div>
  );
}
