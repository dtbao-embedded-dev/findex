import { useState } from "react";

import type { GreetingState } from "../presenter/greetingPresenter";

type Props = {
  state: GreetingState;
  onSubmit: (name: string) => void;
};

/** Rendering only: the view reports input and shows state, never calls the model. */
export function GreetingView({ state, onSubmit }: Props) {
  const [name, setName] = useState("");

  return (
    <>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit(name);
        }}
      >
        <input
          aria-label="Your name"
          placeholder="Your name"
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
        <button type="submit">Greet</button>
      </form>
      <p role="status" className={state.error === null ? undefined : "error"}>
        {state.error ?? state.message}
      </p>
    </>
  );
}
