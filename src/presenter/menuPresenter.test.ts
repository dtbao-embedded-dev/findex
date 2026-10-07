import { describe, expect, it } from "vitest";

import type { Hit } from "../model/search";
import { menuFor, placeMenu, runAction, type ActionDeps } from "./menuPresenter";

const folder: Hit = { name: "findex", path: "E:\\Baotd\\findex", isDir: true };
const file: Hit = { name: "notes.md", path: "E:\\Baotd\\notes.md", isDir: false };

/** Fakes for every model call, recording what each received. */
function recorder(fail: Partial<Record<keyof ActionDeps, string>> = {}) {
  const calls: string[] = [];
  const fake =
    (name: keyof ActionDeps) =>
    async (arg: string): Promise<void> => {
      calls.push(`${name} ${arg}`);
      if (fail[name]) {
        throw fail[name];
      }
    };
  const deps: ActionDeps = {
    openClaude: fake("openClaude"),
    openFolder: fake("openFolder"),
    openTerminal: fake("openTerminal"),
    openVsCode: fake("openVsCode"),
    reveal: fake("reveal"),
    copy: fake("copy"),
  };
  return { calls, deps };
}

describe("menuFor", () => {
  it("offers Claude, VS Code, Explorer, a terminal and copying for a folder", () => {
    expect(menuFor(folder).map((item) => item.label)).toEqual([
      "Open with Claude",
      "Open with VS Code",
      "Open folder",
      "Open in Terminal",
      "Copy path",
      "Copy name",
      "Tags…",
    ]);
  });

  it("offers VS Code, showing and copying for a file", () => {
    expect(menuFor(file).map((item) => item.label)).toEqual([
      "Open with VS Code",
      "Show in Explorer",
      "Copy path",
      "Copy name",
    ]);
  });

  it("separates the open actions from the copy actions", () => {
    const groups = menuFor(folder).map((item) => item.group);
    expect(groups).toEqual([0, 0, 0, 0, 1, 1, 2]);
  });
});

describe("runAction", () => {
  it("sends each action to the model with the right text", async () => {
    const { calls, deps } = recorder();
    for (const action of ["claude", "folder", "terminal", "copyPath", "copyName"] as const) {
      await runAction(action, folder, deps);
    }
    await runAction("reveal", file, deps);
    await runAction("vscode", folder, deps);
    await runAction("vscode", file, deps);
    expect(calls).toEqual([
      "openClaude E:\\Baotd\\findex",
      "openFolder E:\\Baotd\\findex",
      "openTerminal E:\\Baotd\\findex",
      "copy E:\\Baotd\\findex",
      "copy findex",
      "reveal E:\\Baotd\\notes.md",
      "openVsCode E:\\Baotd\\findex",
      "openVsCode E:\\Baotd\\notes.md",
    ]);
  });

  it("confirms a copy, since nothing on screen changes", async () => {
    const { deps } = recorder();
    expect(await runAction("copyPath", folder, deps)).toEqual({
      notice: "Path copied",
      error: null,
    });
    expect(await runAction("copyName", folder, deps)).toEqual({
      notice: "Name copied",
      error: null,
    });
    expect(await runAction("terminal", folder, deps)).toEqual({ notice: null, error: null });
  });

  it("returns the model's refusal as the error", async () => {
    const { deps } = recorder({ openClaude: "Claude Code is not installed" });
    expect(await runAction("claude", folder, deps)).toEqual({
      notice: null,
      error: "Claude Code is not installed",
    });
  });
});

describe("placeMenu", () => {
  it("opens down and right of the pointer when it fits", () => {
    expect(placeMenu({ x: 100, y: 50 }, { width: 200, height: 150 }, { width: 800, height: 600 }))
      .toEqual({ left: 100, top: 50, origin: "top left" });
  });

  it("flips left or up at the window's edges, growing from the pointer", () => {
    expect(placeMenu({ x: 700, y: 500 }, { width: 200, height: 150 }, { width: 800, height: 600 }))
      .toEqual({ left: 500, top: 350, origin: "bottom right" });
  });

  it("never leaves the window, even when it is too small", () => {
    expect(placeMenu({ x: 50, y: 20 }, { width: 200, height: 150 }, { width: 120, height: 100 }))
      .toEqual({ left: 0, top: 0, origin: "top left" });
  });
});
