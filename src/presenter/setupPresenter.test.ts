import { describe, expect, it } from "vitest";

import type { Drive } from "../model/drives";
import {
  canContinue,
  defaultSelection,
  describeDrive,
  formatBytes,
  initialSetup,
  loadSetup,
  needsSetup,
  saveSetup,
  toggleDrive,
} from "./setupPresenter";

const GB = 1024 ** 3;

function drive(letter: string, more: Partial<Drive> = {}): Drive {
  return {
    letter,
    label: "",
    filesystem: "NTFS",
    removable: false,
    total: 512 * GB,
    free: 128 * GB,
    method: "mft",
    ...more,
  };
}

describe("needsSetup", () => {
  it("is true until a drive list was saved", async () => {
    expect(await needsSetup(async () => null)).toBe(true);
    expect(await needsSetup(async () => ["C"])).toBe(false);
  });

  it("asks again when the saved list cannot be read", async () => {
    expect(await needsSetup(async () => Promise.reject("unreadable"))).toBe(true);
  });
});

describe("loadSetup", () => {
  it("preselects the system drive on first run", async () => {
    const state = await loadSetup(
      async () => [drive("D"), drive("C")],
      async () => null,
    );
    expect(state.selected).toEqual(["C"]);
  });

  it("restores a saved choice, dropping drives not plugged in", async () => {
    const state = await loadSetup(
      async () => [drive("C"), drive("D")],
      async () => ["D", "E"],
    );
    expect(state.selected).toEqual(["D"]);
  });

  it("reports a failed listing", async () => {
    const state = await loadSetup(
      async () => Promise.reject("no drives"),
      async () => null,
    );
    expect(state.error).toBe("no drives");
  });
});

describe("selection", () => {
  it("falls back to the first fixed NTFS drive without a C:", () => {
    expect(defaultSelection([drive("E", { removable: true }), drive("F"), drive("G")])).toEqual([
      "F",
    ]);
    expect(defaultSelection([])).toEqual([]);
  });

  it("toggles letters in order and blocks continuing with none", () => {
    let state = { ...initialSetup, drives: [drive("C"), drive("D")], selected: ["D"] };
    state = toggleDrive(state, "C");
    expect(state.selected).toEqual(["C", "D"]);
    state = toggleDrive(toggleDrive(state, "C"), "D");
    expect(state.selected).toEqual([]);
    expect(canContinue(state)).toBe(false);
  });
});

describe("saveSetup", () => {
  it("stores the choice through the model", async () => {
    let stored: string[] = [];
    const result = await saveSetup({ ...initialSetup, selected: ["C", "D"] }, async (l) => {
      stored = l;
      return l;
    });
    expect(stored).toEqual(["C", "D"]);
    expect(result.saved).toBe(true);
  });

  it("refuses an empty choice without calling the model", async () => {
    let called = false;
    const result = await saveSetup(initialSetup, async (l) => {
      called = true;
      return l;
    });
    expect(result.saved).toBe(false);
    expect(result.state.error).toBe("Choose at least one drive.");
    expect(called).toBe(false);
  });

  it("shows the model's refusal", async () => {
    const result = await saveSetup({ ...initialSetup, selected: ["C"] }, async () =>
      Promise.reject("choose at least one drive"),
    );
    expect(result).toMatchObject({ saved: false, state: { error: "choose at least one drive" } });
  });
});

describe("describeDrive", () => {
  it("words size, label and method like Explorer", () => {
    expect(describeDrive(drive("C", { label: "Windows" }))).toEqual({
      title: "Windows (C:)",
      detail: "128 GB free of 512 GB",
      method: "NTFS · instant, stays current",
      usedPercent: 75,
    });
    expect(
      describeDrive(drive("E", { removable: true, filesystem: "exFAT", method: "walk" })).title,
    ).toBe("USB Drive (E:)");
  });

  it("formats bytes in powers of 1024", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(1.8 * 1024 ** 4)).toBe("1.8 TB");
    expect(formatBytes(512 * GB)).toBe("512 GB");
  });
});
