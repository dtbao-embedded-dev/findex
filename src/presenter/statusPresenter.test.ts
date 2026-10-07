import { describe, expect, it } from "vitest";

import type { DriveStatus, IndexStatus } from "../model/index";
import { summarize, watchStatus, type StatusLine } from "./statusPresenter";

/** A drive status; a ready drive's entries are all searchable unless said otherwise. */
function drive(letter: string, more: Partial<DriveStatus> = {}): DriveStatus {
  const state = more.state ?? "ready";
  const entries = more.entries ?? 0;
  return {
    letter,
    state,
    entries,
    searchable: state === "ready" ? entries : 0,
    method: "mft",
    note: null,
    ...more,
  };
}

describe("summarize", () => {
  it("shows progress per drive while indexing", () => {
    const line = summarize({
      drives: [drive("C", { state: "indexing", entries: 120000 }), drive("E", { entries: 5 })],
    });
    // Only what can be searched now counts: C: has no snapshot yet.
    expect(line).toMatchObject({ text: "Indexing C: 120,000", busy: true, total: 5 });
  });

  it("counts a snapshot as searchable while its drive re-indexes", () => {
    const line = summarize({
      drives: [drive("C", { state: "indexing", entries: 10, searchable: 4000000 })],
    });
    expect(line.total).toBe(4000000);
  });

  it("totals the ready drives and names failed ones", () => {
    const line = summarize({
      drives: [
        drive("C", { entries: 2000000 }),
        drive("E", { entries: 1500 }),
        drive("F", { state: "failed", note: "device not ready" }),
      ],
    });
    expect(line.text).toBe("2,001,500 items on C: E: · F: failed");
    expect(line.notes).toEqual(["F: device not ready"]);
  });

  it("says when no drive is chosen", () => {
    expect(summarize({ drives: [] }).text).toBe("No drives chosen");
  });
});

describe("watchStatus", () => {
  it("shows the first read, then every event", async () => {
    const lines: StatusLine[] = [];
    let send: (status: IndexStatus) => void = () => {};
    await watchStatus(
      (line) => lines.push(line),
      async () => ({ drives: [drive("C", { entries: 1 })] }),
      async (cb) => {
        send = cb;
        return () => {};
      },
    );
    send({ drives: [drive("C", { entries: 2 })] });
    expect(lines.map((l) => l.total)).toEqual([1, 2]);
  });

  it("skips a first read that lost the race to an event", async () => {
    const lines: StatusLine[] = [];
    await watchStatus(
      (line) => lines.push(line),
      async () => ({ drives: [drive("C", { entries: 1 })] }),
      async (cb) => {
        cb({ drives: [drive("C", { entries: 9 })] });
        return () => {};
      },
    );
    expect(lines.map((l) => l.total)).toEqual([9]);
  });
});
