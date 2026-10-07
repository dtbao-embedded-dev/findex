import { describe, expect, it } from "vitest";

import type { Hit } from "../model/search";
import {
  describeResults,
  folderOf,
  highlight,
  latestOnly,
  moveSelection,
  revealHit,
} from "./searchPresenter";

function hit(path: string, isDir = false): Hit {
  return { name: path.slice(path.lastIndexOf("\\") + 1), path, isDir };
}

/** A model search whose answers the test releases in any order. */
function deferredSearch() {
  const pending = new Map<string, (hits: Hit[] | null) => void>();
  const ask = (query: string) =>
    new Promise<Hit[] | null>((resolve) => {
      pending.set(query, resolve);
    });
  return { ask, answer: (query: string, hits: Hit[] | null) => pending.get(query)?.(hits) };
}

describe("latestOnly", () => {
  it("drops an older answer that arrives after a newer one", async () => {
    const model = deferredSearch();
    const search = latestOnly(model.ask, () => 0);
    const first = search("re");
    const second = search("report");
    model.answer("report", [hit("C:\\report.pdf")]);
    model.answer("re", [hit("C:\\readme.md")]);
    expect(await second).toMatchObject({ hits: [hit("C:\\report.pdf")], selected: 0 });
    expect(await first).toBeNull();
  });

  it("treats a search Rust cancelled as superseded", async () => {
    const search = latestOnly(async () => null);
    expect(await search("x")).toBeNull();
  });

  it("clears the results for an empty query without asking", async () => {
    let asked = false;
    const search = latestOnly(async () => {
      asked = true;
      return [];
    });
    expect(await search("   ")).toEqual({ hits: [], selected: 0, error: null, tookMs: null });
    expect(asked).toBe(false);
  });

  it("reports how long the search took", async () => {
    let clock = 100;
    const search = latestOnly(
      async () => {
        clock = 104;
        return [];
      },
      () => clock,
    );
    expect((await search("a"))?.tookMs).toBe(4);
  });

  it("shows the latest search's error", async () => {
    const search = latestOnly(async () => Promise.reject("index unavailable"));
    expect(await search("a")).toMatchObject({ error: "index unavailable", hits: [] });
  });
});

describe("describeResults", () => {
  it("counts results and says when the page is full", () => {
    expect(describeResults(1, 0.4)).toBe("1 result · 1 ms");
    expect(describeResults(12, 3.6)).toBe("12 results · 4 ms");
    expect(describeResults(200, 15)).toBe("Top 200 results · 15 ms");
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
