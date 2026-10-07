import { useEffect, useState } from "react";

import { initialGreeting, submitName } from "../presenter/greetingPresenter";
import { needsSetup } from "../presenter/setupPresenter";
import { revealMain } from "../presenter/splashPresenter";
import { GreetingView } from "./GreetingView";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "../model/window";
import { SetupView } from "./SetupView";
import { TitleBar } from "./TitleBar";

type Screen = "loading" | "setup" | "search";

/**
 * The root component. It holds view state and hands user input to the presenter;
 * deciding what that input means is the presenter's job, not the component's.
 */
export function App() {
  const [greeting, setGreeting] = useState(initialGreeting);
  const [screen, setScreen] = useState<Screen>("loading");
  const [firstRun, setFirstRun] = useState(false);

  useEffect(() => {
    const route = needsSetup().then((setup) => {
      setFirstRun(setup);
      setScreen(setup ? "setup" : "search");
    });
    void revealMain(route);
  }, []);

  return (
    <>
      <TitleBar
        title="Findex"
        onMinimize={minimizeWindow}
        onToggleMaximize={toggleMaximizeWindow}
        onClose={closeWindow}
      />
      {screen === "setup" && (
        <SetupView
          firstRun={firstRun}
          onDone={() => {
            setFirstRun(false);
            setScreen("search");
          }}
          onCancel={() => setScreen("search")}
        />
      )}
      {screen === "search" && (
        <>
          <nav className="toolbar">
            <button type="button" className="button-secondary" onClick={() => setScreen("setup")}>
              Drives
            </button>
          </nav>
          <main>
            <h1>Findex</h1>
            <GreetingView
              state={greeting}
              onSubmit={(name) => {
                void submitName(name).then(setGreeting);
              }}
            />
          </main>
        </>
      )}
    </>
  );
}
