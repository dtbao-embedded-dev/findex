import { searchContents, type Batch } from "../model/contents";
import { cancelSearch, type Filters, type Hit } from "../model/search";
import {
  canSearch,
  CLEARED,
  type ScanProgress,
  type SearchUpdate,
} from "./searchPresenter";

/** What one step of a content search changes on screen. */
export type ScanUpdate = Partial<SearchUpdate>;

type Ask = (query: string, filters: Filters, onBatch: (batch: Batch) => void) => Promise<void>;

/** How long typing must pause before a content search reads the disk. */
const PAUSE_MS = 250;

/** The fewest milliseconds between two updates of the list while a scan runs. */
const REDRAW_MS = 100;

/**
 * Runs content searches so only the latest one reaches the screen. Each keystroke
 * restarts the wait, so a search starts once typing pauses rather than reading the
 * disk for every letter. The results grow as batches arrive; the list is redrawn at
 * most every 100 ms while the scan runs, and once more when it ends. `stop` drops
 * what is still on its way, as when the search goes back to names. Stopping, or
 * clearing the field, also stops a scan still running in Rust, which would otherwise
 * read the drive on with nobody waiting.
 */
export function latestScan(
  ask: Ask = searchContents,
  {
    now = () => performance.now(),
    wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms)),
    cancel = cancelSearch,
  }: {
    now?: () => number;
    wait?: (ms: number) => Promise<void>;
    cancel?: () => Promise<void>;
  } = {},
): {
  start: (query: string, filters: Filters, onUpdate: (update: ScanUpdate) => void) => Promise<void>;
  stop: () => void;
} {
  let latest = 0;
  // The scan Rust is running for us, if any; only that one needs stopping there.
  let running: number | null = null;
  const halt = () => {
    if (running !== null) {
      running = null;
      void cancel().catch(() => {});
    }
  };
  const start = async (
    query: string,
    filters: Filters,
    onUpdate: (update: ScanUpdate) => void,
  ): Promise<void> => {
    const mine = ++latest;
    if (!canSearch(query, filters)) {
      halt();
      onUpdate(CLEARED);
      return;
    }
    const begun: ScanProgress = { filesRead: 0, filesTotal: 0, done: false };
    onUpdate({ ...CLEARED, scan: begun });
    await wait(PAUSE_MS);
    if (mine !== latest) {
      return;
    }
    const started = now();
    // Grown in place and copied only when drawn: a long scan sends thousands of batches.
    const hits: Hit[] = [];
    const seen = new Set<string>();
    let drawn = -Infinity;
    running = mine;
    try {
      await ask(query, filters, (batch) => {
        if (mine !== latest) {
          return;
        }
        for (const hit of batch.hits) {
          if (!seen.has(hit.path)) {
            seen.add(hit.path);
            hits.push(hit);
          }
        }
        const at = now();
        if (!batch.done && at - drawn < REDRAW_MS) {
          return;
        }
        drawn = at;
        const scan = { filesRead: batch.filesRead, filesTotal: batch.filesTotal, done: batch.done };
        onUpdate({
          hits: hits.slice(),
          scan,
          total: batch.done ? hits.length : null,
          tookMs: batch.done ? at - started : null,
        });
      });
    } catch (err) {
      if (mine === latest) {
        onUpdate({ ...CLEARED, error: String(err) });
      }
    } finally {
      if (running === mine) {
        running = null;
      }
    }
  };
  const stop = () => {
    latest += 1;
    halt();
  };
  return { start, stop };
}

const count = (n: number) => n.toLocaleString("en-US");

/**
 * The line above the results of a content search: while it runs, how many results so
 * far and how far the reading got; once done, the results, the files read and the
 * time it took.
 */
export function describeScan(found: number, scan: ScanProgress, tookMs: number | null): string {
  const results = `${count(found)} result${found === 1 ? "" : "s"}`;
  if (!scan.done) {
    const parts = [
      found === 0 ? "" : results,
      scan.filesTotal === 0 ? "" : `${count(scan.filesRead)} of ${count(scan.filesTotal)} files read`,
    ].filter((part) => part !== "");
    return ["Searching contents…", parts.join(" · ")].join(parts.length === 0 ? "" : " ");
  }
  const ms = tookMs ?? 0;
  const time = ms < 1000 ? `${Math.max(1, Math.round(ms))} ms` : `${(ms / 1000).toFixed(1)} s`;
  return `${results} · ${count(scan.filesRead)} files read · ${time}`;
}
