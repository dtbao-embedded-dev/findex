import { useEffect } from "react";

import type { UpdateState } from "../presenter/updatePresenter";

type Props = {
  state: UpdateState;
  onRestart: () => void;
  onLater: () => void;
};

/**
 * A small card above the status bar: an update ready to install, or the answer to
 * a check asked for from the tray. An answer fades away on its own.
 */
export function UpdateBanner({ state, onRestart, onLater }: Props) {
  useEffect(() => {
    if (state.kind !== "message") {
      return;
    }
    const timer = setTimeout(onLater, state.tone === "error" ? 8000 : 4000);
    return () => clearTimeout(timer);
  }, [state, onLater]);

  if (state.kind === "idle") {
    return null;
  }
  return (
    <div
      className={
        state.kind === "message" && state.tone === "error" ? "update-banner is-error" : "update-banner"
      }
      role="status"
    >
      {state.kind === "checking" && <span>Checking for updates…</span>}
      {state.kind === "message" && <span>{state.text}</span>}
      {state.kind === "ready" && (
        <>
          <span>
            Findex <strong>{state.version}</strong> is ready to install.
          </span>
          <button type="button" className="is-quiet" onClick={onLater}>
            Later
          </button>
          <button type="button" onClick={onRestart}>
            Restart now
          </button>
        </>
      )}
    </div>
  );
}
