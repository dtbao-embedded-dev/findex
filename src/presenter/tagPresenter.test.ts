import { describe, expect, it } from "vitest";

import type { TagCount } from "../model/tags";
import { addTagTo, loadTags, removeTagFrom, suggestTags, tagQuery } from "./tagPresenter";

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

describe("tags in use", () => {
  const all: TagCount[] = [
    { tag: "work", folders: 3 },
    { tag: "fw", folders: 2 },
    { tag: "firmware", folders: 1 },
  ];

  it("lists them, or none when they cannot be read", async () => {
    expect(await loadTags(async () => all)).toEqual(all);
    expect(await loadTags(async () => Promise.reject("busy"))).toEqual([]);
  });

  it("suggests tags the folder lacks, matching what is typed", () => {
    expect(suggestTags(all, ["work"], "")).toEqual(["fw", "firmware"]);
    expect(suggestTags(all, [], "#F")).toEqual(["fw", "firmware"]);
    expect(suggestTags(all, ["fw"], "fi")).toEqual(["firmware"]);
  });

  it("searches a tag with the tag filter", () => {
    expect(tagQuery("work")).toBe("tag:work");
  });
});
