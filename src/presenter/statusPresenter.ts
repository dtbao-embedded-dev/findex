import {
  getIndexStatus,
  onIndexStatus,
  rescanDrive,
  type IndexStatus,
} from "../model/index";

/** What the status bar shows. */
export type StatusLine = {
  text: string;
  busy: boolean;
  /** Names that can be searched now, including a drive's snapshot while it re-indexes. */
  total: number;
  /** Notes worth a glance, such as a drive read by folder scan instead of its table. */
  notes: string[];
};

const count = (n: number) => n.toLocaleString("en-US");

/** Words the index status for one line at the bottom of the window. */
export function summarize(status: IndexStatus): StatusLine {
  const drives = status.drives;
  const total = drives.reduce((sum, d) => sum + d.searchable, 0);
  const notes = drives.flatMap((d) => (d.note ? [`${d.letter}: ${d.note}`] : []));
  if (drives.length === 0) {
    return { text: "No drives chosen", busy: false, total: 0, notes };
  }
  const indexing = drives.filter((d) => d.state === "indexing");
  if (indexing.length > 0) {
    const parts = indexing.map((d) => `${d.letter}: ${count(d.entries)}`);
    return { text: `Indexing ${parts.join(" · ")}`, busy: true, total, notes };
  }
  const failed = drives.filter((d) => d.state === "failed").map((d) => `${d.letter}:`);
  const ready = drives.filter((d) => d.state === "ready").map((d) => `${d.letter}:`);
  const text =
    `${count(total)} items` +
    (ready.length > 0 ? ` on ${ready.join(" ")}` : "") +
    (failed.length > 0 ? ` · ${failed.join(" ")} failed` : "");
  return { text, busy: false, total, notes };
}

/**
 * Reports the current status, then every change. The first read is skipped if an
 * event already arrived, so a slow read never replaces newer progress. Resolves to
 * the unsubscribe call.
 */
export async function watchStatus(
  onChange: (line: StatusLine) => void,
  read: () => Promise<IndexStatus> = getIndexStatus,
  listen: (cb: (status: IndexStatus) => void) => Promise<() => void> = onIndexStatus,
): Promise<() => void> {
  let heard = false;
  const unlisten = await listen((status) => {
    heard = true;
    onChange(summarize(status));
  });
  try {
    const status = await read();
    if (!heard) {
      onChange(summarize(status));
    }
  } catch {
    // The events still arrive; the first one fills the bar.
  }
  return unlisten;
}

/**
 * Starts reading the active drive again from scratch; progress then shows in the
 * status bar like any indexing. Resolves to the error to show, or null.
 */
export async function rescan(start: () => Promise<string> = rescanDrive): Promise<string | null> {
  try {
    await start();
    return null;
  } catch (err) {
    return String(err);
  }
}

/** The re-scan button's tooltip: what it re-reads, or that a read is running. */
export function rescanLabel(active: string | null, busy: boolean): string {
  if (busy) {
    return "Indexing…";
  }
  return active === null ? "Re-scan the drive from scratch" : `Re-scan ${active}: from scratch`;
}
