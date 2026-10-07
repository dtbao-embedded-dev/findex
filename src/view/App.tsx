import { useState } from "react";

import { initialGreeting, submitName } from "../presenter/greetingPresenter";
import { GreetingView } from "./GreetingView";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "../model/window";
import { TitleBar } from "./TitleBar";

/**
 * The root component. It holds view state and hands user input to the presenter;
 * deciding what that input means is the presenter's job, not the component's.
 */
export function App() {
  const [greeting, setGreeting] = useState(initialGreeting);

  return (
    <>
      <TitleBar
        title="Findex"
        onMinimize={minimizeWindow}
        onToggleMaximize={toggleMaximizeWindow}
        onClose={closeWindow}
      />
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
  );
}
