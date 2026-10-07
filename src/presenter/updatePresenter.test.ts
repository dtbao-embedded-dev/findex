import { describe, expect, it } from "vitest";

import type { Downloaded } from "../model/updater";
import {
  CHECK_EVERY_MS,
  FIRST_CHECK_MS,
  createUpdater,
  scheduleChecks,
  updateButton,
  type UpdateState,
} from "./updatePresenter";

/** An updater over fakes, recording every state it shows and every model call. */
function harness(fetch: () => Promise<Downloaded | null>, saveFails = false) {
  const states: UpdateState[] = [];
  const calls: string[] = [];
  const updater = createUpdater({
    fetch,
    save: async () => {
      calls.push("save");
      if (saveFails) {
        throw "disk full";
      }
    },
    onState: (state) => states.push(state),
  });
  return { updater, states, calls, last: () => states[states.length - 1] };
}

function downloaded(version: string, calls: string[], fails = false): Downloaded {
  return {
    version,
    install: async () => {
      calls.push(`install ${version}`);
      if (fails) {
        throw "signature mismatch";
      }
    },
  };
}

describe("a scheduled check", () => {
  it("downloads a newer version, then offers a restart", async () => {
    const calls: string[] = [];
    const h = harness(async () => downloaded("0.2.0", calls));
    await h.updater.check(false);
    expect(h.states).toEqual([{ kind: "ready", version: "0.2.0" }]);
  });

  it("stays silent when current or when the check fails", async () => {
    const current = harness(async () => null);
    await current.updater.check(false);
    expect(current.last()).toEqual({ kind: "idle" });

    const offline = harness(async () => Promise.reject("offline"));
    await offline.updater.check(false);
    expect(offline.last()).toEqual({ kind: "idle" });
  });

  it("downloads only once, and shows the offer again later", async () => {
    let fetches = 0;
    const calls: string[] = [];
    const h = harness(async () => {
      fetches += 1;
      return downloaded("0.2.0", calls);
    });
    await h.updater.check(false);
    h.updater.later();
    // Put away, but still downloaded: the title bar keeps a mark for it.
    expect(h.last()).toEqual({ kind: "deferred", version: "0.2.0" });
    await h.updater.check(false);
    expect(fetches).toBe(1);
    expect(h.last()).toEqual({ kind: "ready", version: "0.2.0" });
  });
});

describe("a check asked for from the tray", () => {
  it("says so when this version is current", async () => {
    const h = harness(async () => null);
    await h.updater.check(true);
    expect(h.states).toContainEqual({ kind: "checking" });
    expect(h.last()).toEqual({ kind: "message", text: "Findex is up to date.", tone: "info" });
  });

  it("shows why the check failed", async () => {
    const h = harness(async () => Promise.reject("offline"));
    await h.updater.check(true);
    expect(h.last()).toEqual({
      kind: "message",
      text: "Could not check for updates: offline",
      tone: "error",
    });
  });

  it("answers a tray check that arrives while a background check runs", async () => {
    let fetches = 0;
    let release: (value: Downloaded | null) => void = () => {};
    const h = harness(() => {
      fetches += 1;
      return new Promise((resolve) => (release = resolve));
    });
    const quiet = h.updater.check(false);
    const loud = h.updater.check(true);
    release(null);
    await Promise.all([quiet, loud]);
    expect(fetches).toBe(1);
    expect(h.states).toEqual([
      { kind: "checking" },
      { kind: "message", text: "Findex is up to date.", tone: "info" },
    ]);
  });
});

describe("restart", () => {
  it("saves the index, then installs", async () => {
    const calls: string[] = [];
    const h = harness(async () => downloaded("0.2.0", calls));
    await h.updater.check(false);
    await h.updater.restart();
    expect([...h.calls, ...calls]).toEqual(["save", "install 0.2.0"]);
  });

  it("installs even when saving the index fails: it is rebuilt after", async () => {
    const calls: string[] = [];
    const h = harness(async () => downloaded("0.2.0", calls), true);
    await h.updater.check(false);
    await h.updater.restart();
    expect(calls).toEqual(["install 0.2.0"]);
  });

  it("keeps the offer and shows the error when installing fails", async () => {
    const calls: string[] = [];
    const h = harness(async () => downloaded("0.2.0", calls, true));
    await h.updater.check(false);
    await h.updater.restart();
    expect(h.last()).toEqual({
      kind: "message",
      text: "Could not install the update: signature mismatch",
      tone: "error",
    });
  });

  it("installs once when Restart is pressed twice", async () => {
    const calls: string[] = [];
    const h = harness(async () => downloaded("0.2.0", calls));
    await h.updater.check(false);
    await Promise.all([h.updater.restart(), h.updater.restart()]);
    expect(calls).toEqual(["install 0.2.0"]);
  });

  it("does nothing before an update is ready", async () => {
    const h = harness(async () => null);
    await h.updater.restart();
    expect(h.calls).toEqual([]);
  });
});

describe("scheduleChecks", () => {
  it("checks shortly after launch, then every six hours, until stopped", () => {
    const timers: { fn: () => void; ms: number; repeat: boolean }[] = [];
    const cleared: number[] = [];
    const stop = scheduleChecks(() => {}, {
      setTimeout: (fn, ms) => timers.push({ fn, ms, repeat: false }) - 1,
      setInterval: (fn, ms) => timers.push({ fn, ms, repeat: true }) - 1,
      clear: (id) => cleared.push(id),
    });
    expect(timers.map((t) => [t.ms, t.repeat])).toEqual([
      [FIRST_CHECK_MS, false],
      [CHECK_EVERY_MS, true],
    ]);
    expect(FIRST_CHECK_MS).toBe(10_000);
    expect(CHECK_EVERY_MS).toBe(6 * 60 * 60 * 1000);
    stop();
    expect(cleared).toEqual([0, 1]);
  });
});

describe("updateButton", () => {
  it("checks when nothing is waiting, and marks a downloaded update", () => {
    expect(updateButton({ kind: "idle" })).toEqual({
      label: "Check for updates",
      badge: false,
      busy: false,
    });
    expect(updateButton({ kind: "checking" })).toEqual({
      label: "Checking for updates…",
      badge: false,
      busy: true,
    });
    expect(updateButton({ kind: "ready", version: "0.2.0" })).toEqual({
      label: "Findex 0.2.0 is ready to install",
      badge: true,
      busy: false,
    });
    expect(updateButton({ kind: "deferred", version: "0.2.0" }).badge).toBe(true);
    expect(updateButton({ kind: "message", text: "Findex is up to date.", tone: "info" }).badge).toBe(
      false,
    );
  });
});
