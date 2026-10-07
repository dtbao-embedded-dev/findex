import { describe, expect, it } from "vitest";

import {
  dateRange,
  describeDate,
  initialFilters,
  isFiltered,
  toFilters,
  toggleTag,
  type DateChoice,
} from "./filterPresenter";

// 7 Oct 2026, 15:30 local time: a Wednesday in the middle of a month.
const NOW = new Date(2026, 9, 7, 15, 30);
const at = (y: number, m: number, d: number) => new Date(y, m, d).getTime();

describe("dateRange", () => {
  const cases: [DateChoice, number | null, number | null][] = [
    [{ kind: "any" }, null, null],
    [{ kind: "today" }, at(2026, 9, 7), at(2026, 9, 8)],
    [{ kind: "yesterday" }, at(2026, 9, 6), at(2026, 9, 7)],
    [{ kind: "last7" }, at(2026, 9, 1), at(2026, 9, 8)],
    [{ kind: "last30" }, at(2026, 8, 8), at(2026, 9, 8)],
    [{ kind: "thisMonth" }, at(2026, 9, 1), at(2026, 10, 1)],
    [{ kind: "thisYear" }, at(2026, 0, 1), at(2027, 0, 1)],
    [{ kind: "day", date: "2025-02-28" }, at(2025, 1, 28), at(2025, 2, 1)],
    [{ kind: "month", month: "2025-12" }, at(2025, 11, 1), at(2026, 0, 1)],
    [{ kind: "year", year: "2024" }, at(2024, 0, 1), at(2025, 0, 1)],
    [{ kind: "range", from: "2026-01-10", to: "2026-01-12" }, at(2026, 0, 10), at(2026, 0, 13)],
    [{ kind: "range", from: "2026-01-10", to: "" }, at(2026, 0, 10), null],
    [{ kind: "range", from: "", to: "2026-01-12" }, null, at(2026, 0, 13)],
  ];

  it.each(cases)("%j", (choice, from, to) => {
    expect(dateRange(choice, NOW)).toEqual({ from, to });
  });

  it("swaps a range picked back to front", () => {
    expect(dateRange({ kind: "range", from: "2026-01-12", to: "2026-01-10" }, NOW)).toEqual({
      from: at(2026, 0, 10),
      to: at(2026, 0, 13),
    });
  });

  it("narrows nothing until a custom value is complete and valid", () => {
    for (const choice of [
      { kind: "day", date: "" },
      { kind: "day", date: "2026-02-30" },
      { kind: "month", month: "2026-13" },
      { kind: "year", year: "" },
      { kind: "year", year: "26" },
      // Two-digit years would land in the 1900s: anything before 1970 is refused.
      { kind: "year", year: "0050" },
      { kind: "month", month: "0050-03" },
      { kind: "day", date: "0050-03-01" },
      { kind: "range", from: "", to: "" },
    ] as DateChoice[]) {
      expect(dateRange(choice, NOW)).toEqual({ from: null, to: null });
    }
  });
});

describe("describeDate", () => {
  it("names presets and custom choices for the filter button", () => {
    expect(describeDate({ kind: "any" })).toBe("Any date");
    expect(describeDate({ kind: "last7" })).toBe("Last 7 days");
    expect(describeDate({ kind: "day", date: "2026-03-05" })).toBe("Mar 5, 2026");
    expect(describeDate({ kind: "month", month: "2026-03" })).toBe("March 2026");
    expect(describeDate({ kind: "year", year: "2026" })).toBe("2026");
    expect(describeDate({ kind: "range", from: "2026-03-05", to: "2026-03-09" })).toBe(
      "Mar 5, 2026 – Mar 9, 2026",
    );
    expect(describeDate({ kind: "range", from: "2026-03-05", to: "" })).toBe("Since Mar 5, 2026");
    expect(describeDate({ kind: "range", from: "", to: "2026-03-09" })).toBe("Until Mar 9, 2026");
    expect(describeDate({ kind: "month", month: "" })).toBe("Any date");
  });
});

describe("toFilters", () => {
  it("turns the filter bar into what Rust narrows by", () => {
    const filters = toFilters(
      { date: { kind: "today" }, kind: "folder", tags: ["work"] },
      NOW,
    );
    expect(filters).toEqual({
      kind: "folder",
      tags: ["work"],
      modifiedFrom: at(2026, 9, 7),
      modifiedTo: at(2026, 9, 8),
    });
    expect(toFilters(initialFilters, NOW)).toEqual({
      kind: "any",
      tags: [],
      modifiedFrom: null,
      modifiedTo: null,
    });
  });
});

describe("isFiltered", () => {
  it("is true once any filter narrows", () => {
    expect(isFiltered(initialFilters)).toBe(false);
    expect(isFiltered({ ...initialFilters, kind: "file" })).toBe(true);
    expect(isFiltered({ ...initialFilters, tags: ["x"] })).toBe(true);
    expect(isFiltered({ ...initialFilters, date: { kind: "thisYear" } })).toBe(true);
    // A custom date still being typed narrows nothing yet.
    expect(isFiltered({ ...initialFilters, date: { kind: "day", date: "" } })).toBe(false);
  });
});

describe("toggleTag", () => {
  it("adds a tag that is off and removes one that is on", () => {
    expect(toggleTag(["a"], "b")).toEqual(["a", "b"]);
    expect(toggleTag(["a", "b"], "a")).toEqual(["b"]);
  });
});
