import { describe, expect, it } from "vitest";

import type { Details } from "../model/details";
import type { Answer, Filters, Hit } from "../model/search";
import {
  canSearch,
  describeHit,
  describeResults,
  folderOf,
  highlight,
  keepSelection,
  latestDetails,
  latestOnly,
  moveSelection,
  revealHit,
} from "./searchPresenter";

function hit(path: string, isDir = false): Hit {
  return { name: path.slice(path.lastIndexOf("\\") + 1), path, isDir };
}

const NO_FILTERS: Filters = { kind: "any", tags: [], modifiedFrom: null, modifiedTo: null };

const found = (hits: Hit[], partial = false): Answer => ({ hits, partial });

/** A model search whose answers the test releases in any order, keyed by query and kind. */
function deferredSearch() {
  const pending = new Map<string, (answer: Answer | null) => void>();
  const key = (query: string, filters: Filters) => `${query}|${filters.kind}`;
  const ask = (query: string, filters: Filters) =>
    new Promise<Answer | null>((resolve) => {
      pending.set(key(query, filters), resolve);
    });
  return {
    ask,
    answer: (query: string, hits: Hit[] | null, filters = NO_FILTERS) =>
      pending.get(key(query, filters))?.(hits && found(hits)),
  };
}

describe("latestOnly", () => {
  it("drops an older answer that arrives after a newer one", async () => {
    const model = deferredSearch();
    const search = latestOnly(model.ask, () => 0);
    const first = search("re", NO_FILTERS);
    const second = search("report", NO_FILTERS);
    model.answer("report", [hit("C:\\report.pdf")]);
    model.answer("re", [hit("C:\\readme.md")]);
    expect(await second).toMatchObject({ hits: [hit("C:\\report.pdf")], selected: 0 });
    expect(await first).toBeNull();
  });

  it("drops an answer for filters changed since", async () => {
    const model = deferredSearch();
    const search = latestOnly(model.ask, () => 0);
    const folders = { ...NO_FILTERS, kind: "folder" as const };
    const first = search("re", NO_FILTERS);
    const second = search("re", folders);
    model.answer("re", [hit("C:\\reports", true)], folders);
    model.answer("re", [hit("C:\\readme.md")]);
    expect(await second).toMatchObject({ hits: [hit("C:\\reports", true)] });
    expect(await first).toBeNull();
  });

  it("passes the filters to the model and says when the answer is partial", async () => {
    let asked: Filters | null = null;
    const tagged = { ...NO_FILTERS, tags: ["work"], modifiedFrom: 5 };
    const search = latestOnly(async (_query, filters) => {
      asked = filters;
      return found([hit("C:\\proj", true)], true);
    });
    // A tag alone is enough to ask: it lists the folders carrying it.
    expect(await search("", tagged)).toMatchObject({
      partial: true,
      hits: [hit("C:\\proj", true)],
    });
    expect(asked).toEqual(tagged);
  });

  it("asks again when Rust cancelled the latest search for an older one", async () => {
    // Two searches can reach Rust out of order; the older then cancels the newer.
    let calls = 0;
    const search = latestOnly(async () => (++calls === 1 ? null : found([hit("C:\\x.txt")])));
    expect(await search("x", NO_FILTERS)).toMatchObject({ hits: [hit("C:\\x.txt")] });
    expect(calls).toBe(2);
  });

  it("gives up after one retry", async () => {
    const search = latestOnly(async () => null);
    expect(await search("x", NO_FILTERS)).toBeNull();
  });

  it("clears the results for an empty query without asking", async () => {
    let asked = false;
    const search = latestOnly(async () => {
      asked = true;
      return found([]);
    });
    const cleared = { hits: [], selected: 0, error: null, tookMs: null, partial: false };
    expect(await search("   ", NO_FILTERS)).toEqual(cleared);
    // A kind or a date alone has no name to look for.
    expect(await search("", { ...NO_FILTERS, kind: "file", modifiedFrom: 1 })).toEqual(cleared);
    expect(asked).toBe(false);
  });

  it("reports how long the search took", async () => {
    let clock = 100;
    const search = latestOnly(
      async () => {
        clock = 104;
        return found([]);
      },
      () => clock,
    );
    expect((await search("a", NO_FILTERS))?.tookMs).toBe(4);
  });

  it("shows the latest search's error", async () => {
    const search = latestOnly(async () => Promise.reject("index unavailable"));
    expect(await search("a", NO_FILTERS)).toMatchObject({ error: "index unavailable", hits: [] });
  });
});

describe("describeResults", () => {
  it("counts results and says when the page is full", () => {
    expect(describeResults(1, 0.4)).toBe("1 result · 1 ms");
    expect(describeResults(12, 3.6)).toBe("12 results · 4 ms");
    expect(describeResults(200, 15)).toBe("Top 200 results · 15 ms");
  });

  it("says how to see more when a date was checked on the best matches only", () => {
    expect(describeResults(3, 80, true)).toBe(
      "3 results · 80 ms · type more of the name to find older matches",
    );
  });
});

describe("canSearch", () => {
  it("needs a name, or a tag to list tagged folders", () => {
    expect(canSearch("  ", { tags: [] })).toBe(false);
    expect(canSearch("", { tags: ["work"] })).toBe(true);
    expect(canSearch("rep", { tags: [] })).toBe(true);
  });
});

describe("keepSelection", () => {
  it("stays on the same file when a refresh moves it", () => {
    const hits = [hit("C:\\new.txt"), hit("C:\\a.txt"), hit("C:\\b.txt")];
    expect(keepSelection(hit("C:\\a.txt"), hits)).toBe(1);
  });

  it("falls back to the top when the file left the results", () => {
    expect(keepSelection(hit("C:\\gone.txt"), [hit("C:\\a.txt")])).toBe(0);
    expect(keepSelection(undefined, [hit("C:\\a.txt")])).toBe(0);
  });
});

describe("moveSelection", () => {
  it("wraps at both ends", () => {
    expect(moveSelection(0, -1, 3)).toBe(2);
    expect(moveSelection(2, 1, 3)).toBe(0);
    expect(moveSelection(1, 1, 3)).toBe(2);
    expect(moveSelection(0, 1, 0)).toBe(0);
  });
});

describe("folderOf", () => {
  it("returns the containing folder, keeping a drive root's backslash", () => {
    expect(folderOf("C:\\Users\\Ada\\a.txt")).toBe("C:\\Users\\Ada");
    expect(folderOf("C:\\a.txt")).toBe("C:\\");
  });
});

describe("highlight", () => {
  it("marks every term, ignoring case and filters", () => {
    expect(highlight("Annual Report.pdf", "report ann ext:pdf")).toEqual([
      { text: "Ann", match: true },
      { text: "ual ", match: false },
      { text: "Report", match: true },
      { text: ".pdf", match: false },
    ]);
  });

  it("marks nothing when no term occurs", () => {
    expect(highlight("a.txt", "type:file")).toEqual([{ text: "a.txt", match: false }]);
  });
});

describe("revealHit", () => {
  it("passes the path to the model and reports a refusal", async () => {
    let shown = "";
    expect(
      await revealHit(hit("C:\\a.txt"), async (path) => {
        shown = path;
      }),
    ).toBeNull();
    expect(shown).toBe("C:\\a.txt");
    expect(
      await revealHit(hit("C:\\gone.txt"), async () => Promise.reject("C:\\gone.txt no longer exists")),
    ).toBe("C:\\gone.txt no longer exists");
  });
});

describe("highlight with filters", () => {
  it("marks nothing for a tag filter", () => {
    expect(highlight("work", "tag:work")).toEqual([{ text: "work", match: false }]);
  });
});

describe("describeHit", () => {
  const date = (ms: number) => new Date(ms).toISOString().slice(0, 10);
  const details = (more: Partial<Details>): Details => ({
    path: "C:\a.pdf",
    kind: "PDF Document",
    size: 2.4 * 1024 * 1024,
    modified: Date.UTC(2026, 9, 5),
    tags: [],
    ...more,
  });

  it("words a file's type and size, then its date", () => {
    expect(describeHit(details({}), date)).toEqual({
      kind: "PDF Document · 2.4 MB",
      modified: "2026-10-05",
    });
  });

  it("gives a folder no size", () => {
    expect(describeHit(details({ kind: "File folder", size: null }), date)).toEqual({
      kind: "File folder",
      modified: "2026-10-05",
    });
  });

  it("shows nothing until the details arrive, and no date when there is none", () => {
    expect(describeHit(undefined, date)).toBeNull();
    expect(describeHit(details({ modified: null }), date)?.modified).toBe("");
  });
});

describe("latestDetails", () => {
  it("indexes the details by path", async () => {
    const load = latestDetails(async (paths) => paths.map((path) => ({ path, kind: "x", size: 1, modified: 2, tags: [] })));
    const found = await load([hit("C:\a.txt"), hit("C:\b.txt")]);
    expect(Object.keys(found ?? {})).toEqual(["C:\a.txt", "C:\b.txt"]);
  });

  it("drops an answer that a newer page of results replaced", async () => {
    const pending: ((d: Details[]) => void)[] = [];
    const load = latestDetails(() => new Promise((resolve) => pending.push(resolve)));
    const first = load([hit("C:\old.txt")]);
    const second = load([hit("C:\new.txt")]);
    pending[1]?.([{ path: "C:\new.txt", kind: "x", size: 1, modified: 1, tags: [] }]);
    pending[0]?.([{ path: "C:\old.txt", kind: "x", size: 1, modified: 1, tags: [] }]);
    expect(await first).toBeNull();
    expect(Object.keys((await second) ?? {})).toEqual(["C:\new.txt"]);
  });

  it("answers no details, not an error, when reading them fails", async () => {
    const load = latestDetails(async () => Promise.reject("busy"));
    expect(await load([hit("C:\a.txt")])).toEqual({});
  });

  it("does not ask the disk for an empty page", async () => {
    let calls = 0;
    const load = latestDetails(async () => {
      calls += 1;
      return [];
    });
    expect(await load([])).toEqual({});
    expect(calls).toBe(0);
  });
});
