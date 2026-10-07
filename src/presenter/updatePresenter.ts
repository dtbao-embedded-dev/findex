import { fetchUpdate, onCheckRequested, saveIndex, type Downloaded } from "../model/updater";

/**
 * What the update banner and the title bar's update button show. "deferred" is a
 * downloaded update the user put off with Later: no banner, but the button keeps
 * its mark until it is installed.
 */
export type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "ready"; version: string }
  | { kind: "deferred"; version: string }
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
  // The check in flight, shared by every caller; `loud` once the tray asked for it.
  let pending: Promise<void> | null = null;
  let loud = false;
  let restarting = false;
  let ready: Downloaded | null = null;

  const run = async (): Promise<void> => {
    try {
      ready = await fetch();
      if (ready) {
        onState({ kind: "ready", version: ready.version });
      } else {
        onState(
          loud ? { kind: "message", text: "Findex is up to date.", tone: "info" } : { kind: "idle" },
        );
      }
    } catch (err) {
      onState(
        loud
          ? { kind: "message", text: `Could not check for updates: ${String(err)}`, tone: "error" }
          : { kind: "idle" },
      );
    } finally {
      pending = null;
      loud = false;
    }
  };

  /**
   * Checks now. A check asked for while another runs joins it rather than fetching
   * twice, and still gets its answer.
   */
  const check = (manual: boolean): Promise<void> => {
    // Downloaded already: offer it again rather than fetching it twice.
    if (ready) {
      onState({ kind: "ready", version: ready.version });
      return Promise.resolve();
    }
    if (manual) {
      loud = true;
      onState({ kind: "checking" });
    }
    pending ??= run();
    return pending;
  };

  /**
   * Saves the live index, then installs; a failed save only means a rebuild later.
   * A second press while installing does nothing.
   */
  const restart = async (): Promise<void> => {
    if (!ready || restarting) {
      return;
    }
    restarting = true;
    try {
      await save();
    } catch {
      // The next launch rebuilds or replays the journal instead.
    }
    try {
      await ready.install();
    } catch (err) {
      restarting = false;
      onState({
        kind: "message",
        text: `Could not install the update: ${String(err)}`,
        tone: "error",
      });
    }
  };

  /** Hides the banner; the download is kept and offered again at the next check. */
  const later = (): void =>
    onState(ready ? { kind: "deferred", version: ready.version } : { kind: "idle" });

  return { check, restart, later };
}

/**
 * The title bar's update button: it checks now, or, with an update downloaded,
 * carries a mark and offers it again.
 */
export function updateButton(state: UpdateState): { label: string; badge: boolean; busy: boolean } {
  switch (state.kind) {
    case "checking":
      return { label: "Checking for updates…", badge: false, busy: true };
    case "ready":
    case "deferred":
      return { label: `Findex ${state.version} is ready to install`, badge: true, busy: false };
    default:
      return { label: "Check for updates", badge: false, busy: false };
  }
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
