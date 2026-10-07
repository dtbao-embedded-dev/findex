import { describe, expect, it } from "vitest";

import { detailsWindow, scrollToShow, visibleRange } from "./listPresenter";

describe("visibleRange", () => {
  it("renders the rows in view plus a margin on each side", () => {
    // Rows of 44 px under a 30 px line, 440 px of view scrolled 1000 px down: rows
    // 22 to 32 are in view (970 px into the list), plus 5 on each side.
    expect(visibleRange(1000, 440, 44, 30, 10_000, 5)).toEqual({ first: 17, last: 38 });
  });

  it("stays inside the list at both ends", () => {
    expect(visibleRange(0, 440, 44, 30, 10_000, 5)).toEqual({ first: 0, last: 15 });
    expect(visibleRange(0, 440, 44, 30, 3, 5)).toEqual({ first: 0, last: 3 });
    // Scrolled past the end, as for a moment after the results shrink.
    expect(visibleRange(1_000_000, 440, 44, 30, 100, 5)).toEqual({ first: 100, last: 100 });
    expect(visibleRange(0, 440, 44, 30, 0, 5)).toEqual({ first: 0, last: 0 });
  });
});

describe("scrollToShow", () => {
  // The list starts 30 px down the scroll area; rows are 44 px; 440 px are in view.
  it("leaves the scroll alone when the row is fully in view", () => {
    expect(scrollToShow(3, 44, 30, 0, 440)).toBeNull();
  });

  it("scrolls up to a row above the view, and to the very top for the first row", () => {
    expect(scrollToShow(10, 44, 30, 1000, 440)).toBe(470);
    expect(scrollToShow(0, 44, 30, 1000, 440)).toBe(0);
  });

  it("scrolls down just enough to show a row below the view", () => {
    // Row 20 ends at 30 + 21 * 44 = 954.
    expect(scrollToShow(20, 44, 30, 0, 440)).toBe(954 - 440);
  });
});

describe("detailsWindow", () => {
  it("rounds the rows in view out to whole chunks", () => {
    expect(detailsWindow(16, 38, 10_000, 50)).toEqual({ first: 0, last: 50 });
    expect(detailsWindow(60, 101, 10_000, 50)).toEqual({ first: 50, last: 150 });
    expect(detailsWindow(60, 101, 120, 50)).toEqual({ first: 50, last: 120 });
  });
});
