import { describe, expect, it } from "vitest";

import type { DriveStatus, IndexStatus } from "../model/index";
import { summarize, watchStatus, type StatusLine } from "./statusPresenter";

function drive(letter: string, more: Partial<DriveStatus> = {}): DriveStatus {
  return { letter, state: "ready", entries: 0, method: "mft", note: null, ...more };
}

describe("summarize", () => {
  it("shows progress per drive while indexing", () => {
    const line = summarize({
      drives: [drive("C", { state: "indexing", entries: 120000 }), drive("E", { entries: 5 })],
    });
    // Only the ready drive counts as searchable.
    expect(line).toMatchObject({ text: "Indexing C: 120,000", busy: true, total: 5 });
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
