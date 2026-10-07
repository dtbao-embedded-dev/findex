import { describe, expect, it } from "vitest";

import type { Batch } from "../model/contents";
import type { Filters, Hit } from "../model/search";
import { describeScan, latestScan, type ScanUpdate } from "./contentPresenter";

const NO_FILTERS: Filters = { kind: "any", tags: [], exts: [], modifiedFrom: null, modifiedTo: null };

const hit = (name: string, line?: number): Hit => ({
  name,
  path: `C:\\${name}`,
  isDir: false,
  ...(line === undefined ? {} : { line, snippet: `${name} line ${line}` }),
});

const batch = (hits: Hit[], filesRead = 0, filesTotal = 0, done = false): Batch => ({
  hits,
  filesRead,
  filesTotal,
  done,
});

/** A model scan the test drives by hand: it records each call and its batch handler. */
function manualScan() {
  const calls: { query: string; send: (batch: Batch) => void; finish: () => void }[] = [];
  const ask = (query: string, _filters: Filters, onBatch: (batch: Batch) => void) =>
    new Promise<void>((resolve) => calls.push({ query, send: onBatch, finish: resolve }));
  return { calls, ask };
}

const noWait = { wait: async () => {} };

describe("latestScan", () => {
  it("clears the list, then grows it as batches arrive, without listing a file twice", async () => {
    const model = manualScan();
    let clock = 0;
    const updates: ScanUpdate[] = [];
    const scan = latestScan(model.ask, { ...noWait, now: () => clock });
    const running = scan.start("spi", NO_FILTERS, (u) => updates.push(u));
    await Promise.resolve();
    const call = model.calls[0]!;
    call.send(batch([hit("spi.c")]));
    clock = 200;
    call.send(batch([hit("spi.c"), hit("main.c", 3)], 10, 40));
    clock = 1_500;
    call.send(batch([hit("bus.c", 7)], 40, 40, true));
    call.finish();
    await running;

    expect(updates[0]).toMatchObject({ hits: [], selected: 0, tookMs: null, more: false });
    expect(updates[0]?.scan).toEqual({ filesRead: 0, filesTotal: 0, done: false });
    const last = updates[updates.length - 1]!;
    expect(last.hits?.map((h) => h.name)).toEqual(["spi.c", "main.c", "bus.c"]);
    expect(last.hits?.[1]).toMatchObject({ line: 3, snippet: "main.c line 3" });
    expect(last.scan).toEqual({ filesRead: 40, filesTotal: 40, done: true });
    expect(last.tookMs).toBe(1_500);
    expect(last.total).toBe(3);
  });

  it("sends the list at most every 100 ms while scanning, and always at the end", async () => {
    const model = manualScan();
    let clock = 0;
    const updates: ScanUpdate[] = [];
    const scan = latestScan(model.ask, { ...noWait, now: () => clock });
    const running = scan.start("x", NO_FILTERS, (u) => updates.push(u));
    await Promise.resolve();
    const call = model.calls[0]!;
    call.send(batch([hit("a.c", 1)], 1, 9));
    clock = 30;
    call.send(batch([hit("b.c", 1)], 2, 9));
    clock = 60;
    call.send(batch([hit("c.c", 1)], 3, 9, true));
    call.finish();
    await running;
    // The start, the first batch, then the end; the middle one rode along.
    expect(updates.map((u) => u.hits?.length)).toEqual([0, 1, 3]);
  });

  it("ignores the batches of a scan a newer one replaced", async () => {
    const model = manualScan();
    const seen: string[] = [];
    const scan = latestScan(model.ask, noWait);
    void scan.start("old", NO_FILTERS, (u) => u.hits?.forEach((h) => seen.push(h.name)));
    await Promise.resolve();
    void scan.start("new", NO_FILTERS, (u) => u.hits?.forEach((h) => seen.push(h.name)));
    await Promise.resolve();
    model.calls[0]!.send(batch([hit("stale.c", 1)], 1, 1, true));
    model.calls[1]!.send(batch([hit("fresh.c", 1)], 1, 1, true));
    expect(seen).toEqual(["fresh.c"]);
  });

  it("drops every batch once stopped, as when the search goes back to names", async () => {
    const model = manualScan();
    const updates: ScanUpdate[] = [];
    const scan = latestScan(model.ask, noWait);
    void scan.start("x", NO_FILTERS, (u) => updates.push(u));
    await Promise.resolve();
    scan.stop();
    model.calls[0]!.send(batch([hit("late.c", 1)], 1, 1, true));
    expect(updates).toHaveLength(1);
  });

  it("asks only once typing pauses", async () => {
    const model = manualScan();
    const waits: (() => void)[] = [];
    const scan = latestScan(model.ask, {
      wait: () => new Promise<void>((resolve) => waits.push(resolve)),
    });
    void scan.start("u", NO_FILTERS, () => {});
    void scan.start("ua", NO_FILTERS, () => {});
    void scan.start("uart", NO_FILTERS, () => {});
    waits.forEach((resolve) => resolve());
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(model.calls.map((c) => c.query)).toEqual(["uart"]);
  });

  it("clears without asking when there is nothing to look for", async () => {
    const model = manualScan();
    const updates: ScanUpdate[] = [];
    await latestScan(model.ask, noWait).start("  ", NO_FILTERS, (u) => updates.push(u));
    expect(model.calls).toHaveLength(0);
    expect(updates).toEqual([expect.objectContaining({ hits: [], scan: null, tookMs: null })]);
  });

  it("shows why the scan failed", async () => {
    const updates: ScanUpdate[] = [];
    const scan = latestScan(async () => Promise.reject("the content search could not start"), noWait);
    await scan.start("x", NO_FILTERS, (u) => updates.push(u));
    expect(updates[updates.length - 1]).toMatchObject({
      error: "the content search could not start",
      scan: null,
    });
  });
});

describe("describeScan", () => {
  it("says it is still searching, and how far the reading got", () => {
    expect(describeScan(0, { filesRead: 0, filesTotal: 0, done: false }, null)).toBe(
      "Searching contents…",
    );
    expect(describeScan(37, { filesRead: 1_204, filesTotal: 52_310, done: false }, null)).toBe(
      "Searching contents… 37 results · 1,204 of 52,310 files read",
    );
    expect(describeScan(1, { filesRead: 0, filesTotal: 0, done: false }, null)).toBe(
      "Searching contents… 1 result",
    );
    expect(describeScan(0, { filesRead: 2_548, filesTotal: 49_585, done: false }, null)).toBe(
      "Searching contents… 2,548 of 49,585 files read",
    );
  });

  it("sums up a finished scan with its time", () => {
    expect(describeScan(37, { filesRead: 52_310, filesTotal: 52_310, done: true }, 4_230)).toBe(
      "37 results · 52,310 files read · 4.2 s",
    );
    expect(describeScan(1, { filesRead: 3, filesTotal: 3, done: true }, 80)).toBe(
      "1 result · 3 files read · 80 ms",
    );
  });
});
