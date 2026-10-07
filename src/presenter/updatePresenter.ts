import { fetchUpdate, onCheckRequested, saveIndex, type Downloaded } from "../model/updater";

/** What the update banner shows. */
export type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "ready"; version: string }
  | { kind: "message"; text: string; tone: "info" | "error" };

/** The first check waits for the app to settle; then one every six hours. */
export const FIRST_CHECK_MS = 10_000;
export const CHECK_EVERY_MS = 6 * 60 * 60 * 1000;

type Deps = {
  fetch?: () => Promise<Downloaded | null>;
  save?: () => Promise<void>;
  onState: (state: UpdateState) => void;
};

/**
 * The update flow: check, download in the background, then offer a restart. A
 * scheduled check is quiet unless it finds something; a check asked for from the
 * tray always answers, with "up to date" or the reason it failed.
 */
export function createUpdater({ fetch = fetchUpdate, save = saveIndex, onState }: Deps) {
  let running = false;
  let ready: Downloaded | null = null;

  const check = async (manual: boolean): Promise<void> => {
    if (running) {
      return;
    }
    // Downloaded already: offer it again rather than fetching it twice.
    if (ready) {
      onState({ kind: "ready", version: ready.version });
      return;
    }
    running = true;
    if (manual) {
      onState({ kind: "checking" });
    }
    try {
      ready = await fetch();
      if (ready) {
        onState({ kind: "ready", version: ready.version });
      } else {
        onState(
          manual ? { kind: "message", text: "Findex is up to date.", tone: "info" } : { kind: "idle" },
        );
      }
    } catch (err) {
      onState(
        manual
          ? { kind: "message", text: `Could not check for updates: ${String(err)}`, tone: "error" }
          : { kind: "idle" },
      );
    } finally {
      running = false;
    }
  };

  /** Saves the live index, then installs; a failed save only means a rebuild later. */
  const restart = async (): Promise<void> => {
    if (!ready) {
      return;
    }
    try {
      await save();
    } catch {
      // The next launch rebuilds or replays the journal instead.
    }
    try {
      await ready.install();
    } catch (err) {
      onState({
        kind: "message",
        text: `Could not install the update: ${String(err)}`,
        tone: "error",
      });
    }
  };

  /** Hides the banner; the download is kept and offered again at the next check. */
  const later = (): void => onState({ kind: "idle" });

  return { check, restart, later };
}

type Timers = {
  setTimeout: (fn: () => void, ms: number) => number;
  setInterval: (fn: () => void, ms: number) => number;
  clear: (id: number) => void;
};

const WINDOW_TIMERS: Timers = {
  setTimeout: (fn, ms) => window.setTimeout(fn, ms),
  setInterval: (fn, ms) => window.setInterval(fn, ms),
  clear: (id) => {
    window.clearTimeout(id);
    window.clearInterval(id);
  },
};

/** Runs `check` shortly after launch, then every six hours; returns the stop call. */
export function scheduleChecks(check: () => void, timers: Timers = WINDOW_TIMERS): () => void {
  const first = timers.setTimeout(check, FIRST_CHECK_MS);
  const every = timers.setInterval(check, CHECK_EVERY_MS);
  return () => {
    timers.clear(first);
    timers.clear(every);
  };
}

/** Runs `check` whenever the tray asks for one; resolves to the unsubscribe call. */
export function listenForChecks(
  check: () => void,
  listen: (onCheck: () => void) => Promise<() => void> = onCheckRequested,
): Promise<() => void> {
  return listen(check);
}
