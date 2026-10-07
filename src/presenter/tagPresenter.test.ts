import { describe, expect, it } from "vitest";

import type { TagInfo } from "../model/tags";
import {
  addTagTo,
  createTagNamed,
  deletePrompt,
  deleteTagNamed,
  keepKnown,
  loadTags,
  removeTagFrom,
  suggestTags,
  tagColors,
  tagLabel,
  tagName,
} from "./tagPresenter";

describe("addTagTo", () => {
  it("adds the typed tag and returns the folder's tags", async () => {
    const calls: string[] = [];
    const result = await addTagTo("C:\p", "  work ", async (path, tag) => {
      calls.push(`${path} ${tag}`);
      return ["fw", "work"];
    });
    expect(calls).toEqual(["C:\p work"]);
    expect(result).toEqual({ tags: ["fw", "work"], error: null });
  });

  it("ignores an empty entry without asking the model", async () => {
    let calls = 0;
    const result = await addTagTo("C:\p", "   ", async () => {
      calls += 1;
      return [];
    });
    expect(calls).toBe(0);
    expect(result).toEqual({ tags: null, error: null });
  });

  it("shows the model's refusal", async () => {
    const result = await addTagTo("C:\p", "two words", async () =>
      Promise.reject("a tag is one word"),
    );
    expect(result).toEqual({ tags: null, error: "a tag is one word" });
  });
});

describe("removeTagFrom", () => {
  it("returns the tags left, or the error", async () => {
    expect(await removeTagFrom("C:\p", "work", async () => ["fw"])).toEqual({
      tags: ["fw"],
      error: null,
    });
    expect(await removeTagFrom("C:\p", "work", async () => Promise.reject("locked"))).toEqual({
      tags: null,
      error: "locked",
    });
  });
});

describe("the tag catalog", () => {
  const red = { tag: "red", color: "red", preset: true, folders: 1 };
  const blue = { tag: "blue", color: "blue", preset: true, folders: 0 };
  const custom = (tag: string, folders: number): TagInfo => ({ tag, color: null, preset: false, folders });
  const all: TagInfo[] = [red, blue, custom("firmware", 1), custom("fw", 2), custom("work", 3)];

  it("lists it, or nothing when it cannot be read", async () => {
    expect(await loadTags(async () => all)).toEqual(all);
    expect(await loadTags(async () => Promise.reject("busy"))).toEqual([]);
  });

  it("suggests custom tags the folder lacks, matching what is typed", () => {
    // The colour tags are offered as their own row of dots, not as suggestions.
    expect(suggestTags(all, ["work"], "")).toEqual(["firmware", "fw"]);
    expect(suggestTags(all, [], "#F")).toEqual(["firmware", "fw"]);
    expect(suggestTags(all, ["fw"], "fi")).toEqual(["firmware"]);
    expect(suggestTags(all, [], "re")).toEqual([]);
  });

  it("names colour tags by their colour and custom tags with a #", () => {
    const colors = tagColors(all);
    expect(colors).toEqual({ red: "red", blue: "blue" });
    expect(tagLabel("red", colors)).toBe("Red");
    expect(tagLabel("fw", colors)).toBe("#fw");
    expect(tagName("red", colors)).toBe("Red");
    expect(tagName("fw", colors)).toBe("fw");
  });

  it("keeps only the filter tags the catalog still has", () => {
    expect(keepKnown(["fw", "gone", "red"], all)).toEqual(["fw", "red"]);
  });

  it("asks before deleting a tag, saying how many folders lose it", () => {
    expect(deletePrompt(custom("archive", 0))).toBe("Delete #archive?");
    expect(deletePrompt(custom("fw", 1))).toBe("Delete #fw? It comes off 1 folder.");
    expect(deletePrompt(custom("work", 3))).toBe("Delete #work? It comes off 3 folders.");
  });
});

describe("createTagNamed", () => {
  it("creates a custom tag and returns the new catalog", async () => {
    const asked: string[] = [];
    const after = [{ tag: "fw", color: null, preset: false, folders: 0 }];
    expect(
      await createTagNamed(" fw ", async (tag) => {
        asked.push(tag);
        return after;
      }),
    ).toEqual({ catalog: after, error: null });
    expect(asked).toEqual(["fw"]);
  });

  it("ignores an empty entry and shows the model's refusal", async () => {
    let asked = false;
    expect(
      await createTagNamed("  ", async () => {
        asked = true;
        return [];
      }),
    ).toEqual({ catalog: null, error: null });
    expect(asked).toBe(false);
    expect(await createTagNamed("fw", async () => Promise.reject("the tag fw already exists"))).toEqual({
      catalog: null,
      error: "the tag fw already exists",
    });
  });
});

describe("deleteTagNamed", () => {
  it("returns the catalog left, or the error", async () => {
    expect(await deleteTagNamed("fw", async () => [])).toEqual({ catalog: [], error: null });
    expect(await deleteTagNamed("red", async () => Promise.reject("a colour tag"))).toEqual({
      catalog: null,
      error: "a colour tag",
    });
  });
});
