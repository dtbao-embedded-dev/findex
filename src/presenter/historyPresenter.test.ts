import { describe, expect, it } from "vitest";

import {
  HISTORY_MAX,
  forget,
  loadHistory,
  parseHistory,
  remember,
  saveHistory,
} from "./historyPresenter";

describe("remember", () => {
  it("puts the newest query first and keeps each query once, whatever its case", () => {
    expect(remember(["uart", "readme"], "README")).toEqual(["README", "uart"]);
    expect(remember([], "  spec  ")).toEqual(["spec"]);
  });

  it("replaces the newest entry when typing went on from it", () => {
    expect(remember(["rea", "uart"], "readme")).toEqual(["readme", "uart"]);
    // Only the newest: an older query that happens to be a prefix stays.
    expect(remember(["uart", "rea"], "readme")).toEqual(["readme", "uart", "rea"]);
  });

  it("ignores a query too short to be worth offering again", () => {
    const history = ["uart"];
    expect(remember(history, " a ")).toBe(history);
    expect(remember(history, "")).toBe(history);
  });

  it("returns the same list when nothing changes, so nothing is saved", () => {
    const history = ["readme", "uart"];
    expect(remember(history, "readme")).toBe(history);
  });

  it("keeps the newest twenty", () => {
    const full = Array.from({ length: HISTORY_MAX }, (_, i) => `query ${i}`);
    const next = remember(full, "newest");
    expect(next).toHaveLength(HISTORY_MAX);
    expect(next[0]).toBe("newest");
    expect(next).not.toContain(`query ${HISTORY_MAX - 1}`);
  });
});

describe("forget", () => {
  it("removes one entry", () => {
    expect(forget(["readme", "uart"], "readme")).toEqual(["uart"]);
  });
});

describe("parseHistory", () => {
  it("reads a saved list and treats anything else as empty", () => {
    expect(parseHistory('["readme","uart"]')).toEqual(["readme", "uart"]);
    expect(parseHistory(null)).toEqual([]);
    expect(parseHistory("not json")).toEqual([]);
    expect(parseHistory('{"a":1}')).toEqual([]);
    expect(parseHistory('["ok", 3, "", null]')).toEqual(["ok"]);
  });
});

describe("loading and saving", () => {
  it("reads and writes the setting, and never fails the search over it", async () => {
    expect(await loadHistory(async () => '["uart"]')).toEqual(["uart"]);
    expect(
      await loadHistory(async () => {
        throw "no database";
      }),
    ).toEqual([]);
    const written: [string, string][] = [];
    await saveHistory(["uart"], async (key, value) => {
      written.push([key, value]);
    });
    expect(written).toEqual([["search.history", '["uart"]']]);
    await saveHistory(["uart"], async () => {
      throw "read-only";
    });
  });
});
