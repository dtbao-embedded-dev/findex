import { describePaths, type Details } from "../model/details";
import { revealPath, searchNames, type Hit } from "../model/search";
import { formatBytes } from "./drivePresenter";

/** What the search screen shows. */
export type SearchState = {
  query: string;
  hits: Hit[];
  selected: number;
  error: string | null;
  /** How long the last answered search took, end to end, in milliseconds. */
  tookMs: number | null;
};

export const initialSearch: SearchState = {
  query: "",
  hits: [],
  selected: 0,
  error: null,
  tookMs: null,
};

/** The part of the state one search answers; null when a later search superseded it. */
export type SearchUpdate = Pick<SearchState, "hits" | "selected" | "error" | "tookMs">;

/**
 * Wraps the model search so only the latest call's answer is used: typing fires a
 * search per keystroke, and an older, slower answer must never replace a newer one.
 * An empty query clears the results without asking Rust.
 */
export function latestOnly(
  ask: (query: string) => Promise<Hit[] | null> = searchNames,
  now: () => number = () => performance.now(),
): (query: string) => Promise<SearchUpdate | null> {
  let latest = 0;
  return async (query) => {
    const mine = ++latest;
    if (query.trim() === "") {
      return { hits: [], selected: 0, error: null, tookMs: null };
    }
    const start = now();
    try {
      let hits = await ask(query);
      // Rust answers null when it cancelled this search for one that reached it
      // later. Calls can reach Rust out of order, so an older keystroke may have
      // cancelled this, the newest: ask once more, and it then wins.
      if (hits === null && mine === latest) {
        hits = await ask(query);
      }
      if (mine !== latest || hits === null) {
        return null;
      }
      return { hits, selected: 0, error: null, tookMs: now() - start };
    } catch (err) {
      return mine === latest ? { hits: [], selected: 0, error: String(err), tookMs: null } : null;
    }
  };
}

/** Rust's default result limit; a full page means there are more matches. */
export const RESULT_LIMIT = 200;

/** The line above the results: how many, and how fast. */
export function describeResults(count: number, tookMs: number): string {
  const amount =
    count >= RESULT_LIMIT ? `Top ${RESULT_LIMIT} results` : `${count} result${count === 1 ? "" : "s"}`;
  return `${amount} · ${Math.max(1, Math.round(tookMs))} ms`;
}

/**
 * Where the selection belongs after the results were refreshed in the background:
 * on the same file if it is still listed, otherwise the top.
 */
export function keepSelection(previous: Hit | undefined, hits: Hit[]): number {
  if (!previous) {
    return 0;
  }
  return Math.max(0, hits.findIndex((hit) => hit.path === previous.path));
}

/** Moves the selection by `delta`, wrapping at both ends. */
export function moveSelection(selected: number, delta: number, count: number): number {
  if (count === 0) {
    return 0;
  }
  return (((selected + delta) % count) + count) % count;
}

/** The folder a result sits in: `C:\Users\a.txt` → `C:\Users`, `C:\a.txt` → `C:\`. */
export function folderOf(path: string): string {
  const cut = path.lastIndexOf("\\");
  if (cut < 0) {
    return "";
  }
  return cut <= 2 ? path.slice(0, cut + 1) : path.slice(0, cut);
}

/** A run of a name, marked when it is part of what the query matched. */
export type Segment = { text: string; match: boolean };

/**
 * Splits `name` into runs, marking where each query term occurs, so the view can
 * show why a result matched. Filters (`ext:`, `type:`) mark nothing.
 */
export function highlight(name: string, query: string): Segment[] {
  const lower = name.toLowerCase();
  // Folding can change the length of some characters; then offsets would not line up.
  if (lower.length !== name.length) {
    return [{ text: name, match: false }];
  }
  const marks = new Array<boolean>(name.length).fill(false);
  for (const token of query.toLowerCase().split(/\s+/)) {
    if (token === "" || token.startsWith("ext:") || token.startsWith("type:")) {
      continue;
    }
    const at = lower.indexOf(token);
    if (at >= 0) {
      marks.fill(true, at, at + token.length);
    }
  }
  const segments: Segment[] = [];
  for (let i = 0; i < name.length; i++) {
    const match = marks[i] ?? false;
    const last = segments[segments.length - 1];
    if (last && last.match === match) {
      last.text += name[i];
    } else {
      segments.push({ text: name[i] ?? "", match });
    }
  }
  return segments;
}

/** Shows a result in Explorer; resolves to the error to show, or null. */
export async function revealHit(
  hit: Hit,
  reveal: (path: string) => Promise<void> = revealPath,
): Promise<string | null> {
  try {
    await reveal(hit.path);
    return null;
  } catch (err) {
    return String(err);
  }
}

/**
 * Fetches the details of a page of results, after the results are already on
 * screen, so reading the disk never slows a search. Only the latest page's answer
 * is used; a failure only means no details are shown.
 */
export function latestDetails(
  ask: (paths: string[]) => Promise<Details[]> = describePaths,
): (hits: Hit[]) => Promise<Record<string, Details> | null> {
  let latest = 0;
  return async (hits) => {
    const mine = ++latest;
    if (hits.length === 0) {
      return {};
    }
    let found: Details[];
    try {
      found = await ask(hits.map((hit) => hit.path));
    } catch {
      found = [];
    }
    if (mine !== latest) {
      return null;
    }
    return Object.fromEntries(found.map((details) => [details.path, details]));
  };
}

const DATE = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

/**
 * The two short lines at the right of a result: its type and size, then when it last
 * changed. Null until the details arrive.
 */
export function describeHit(
  details: Details | undefined,
  formatDate: (ms: number) => string = (ms) => DATE.format(ms),
): { kind: string; modified: string } | null {
  if (!details) {
    return null;
  }
  return {
    kind: details.size === null ? details.kind : `${details.kind} · ${formatBytes(details.size)}`,
    modified: details.modified === null ? "" : formatDate(details.modified),
  };
}
