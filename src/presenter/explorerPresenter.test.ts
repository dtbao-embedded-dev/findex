import { describe, expect, it } from "vitest";

import type { TagInfo } from "../model/tags";
import { loadExplorerMenu, menuTagsKey, setExplorerMenuTo, syncMenu } from "./explorerPresenter";

const tag = (name: string, folders = 0): TagInfo => ({
  tag: name,
  color: null,
  preset: false,
  folders,
});

describe("Explorer menu setting", () => {
  it("reads whether the menu is on, and says why when it cannot", async () => {
    expect(await loadExplorerMenu(async () => false)).toEqual({ enabled: false, error: null });
    expect(
      await loadExplorerMenu(async () => {
        throw "no database";
      }),
    ).toEqual({ enabled: true, error: "no database" });
  });

  it("turns the menu on or off, keeping the old state when that fails", async () => {
    const asked: boolean[] = [];
    const ok = await setExplorerMenuTo(false, true, async (on) => {
      asked.push(on);
    });
    expect(ok).toEqual({ enabled: false, error: null });
    expect(asked).toEqual([false]);
    const failed = await setExplorerMenuTo(true, false, async () => {
      throw "the Explorer menu needs findex-tag.exe";
    });
    expect(failed).toEqual({ enabled: false, error: "the Explorer menu needs findex-tag.exe" });
  });

  it("rewrites the menu only when the tag names change, not their counts", () => {
    expect(menuTagsKey([tag("doc", 1), tag("fw")])).toBe(menuTagsKey([tag("doc", 9), tag("fw")]));
    expect(menuTagsKey([tag("doc")])).not.toBe(menuTagsKey([tag("doc"), tag("fw")]));
    expect(menuTagsKey([])).toBe("");
  });

  it("reports a failed sync as its message", async () => {
    expect(await syncMenu(async () => {})).toBeNull();
    expect(
      await syncMenu(async () => {
        throw "denied";
      }),
    ).toBe("denied");
  });
});
