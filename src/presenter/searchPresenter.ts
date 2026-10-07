import { describePaths, type Details } from "../model/details";
import { revealPath, searchNames, type Answer, type Filters, type Hit } from "../model/search";
import { formatBytes } from "./drivePresenter";
import { RESULTS_DEFAULT } from "./settingsPresenter";

/** What the search screen shows. */
export type SearchState = {
  query: string;
  hits: Hit[];
  selected: number;
  error: string | null;
  /** How long the last answered search took, end to end, in milliseconds. */
  tookMs: number | null;
  /** A date filter was checked on only the best name matches: more may match. */
  partial: boolean;
};

export const initialSearch: SearchState = {
  query: "",
  hits: [],
  selected: 0,
  error: null,
  tookMs: null,
  partial: false,
};

/** The part of the state one search answers; null when a later search superseded it. */
export type SearchUpdate = Pick<SearchState, "hits" | "selected" | "error" | "tookMs" | "partial">;

const CLEARED: SearchUpdate = { hits: [], selected: 0, error: null, tookMs: null, partial: false };

/**
 * Whether there is anything to ask: the index holds names only, so a search needs a
 * name; a tag alone also works, listing the folders that carry it.
 */
export function canSearch(query: string, filters: { tags: string[] }): boolean {
  return query.trim() !== "" || filters.tags.length > 0;
}

/**
 * What the results area shows: the list (also while the first answer is on its way),
 * the error, the empty screen when there is nothing to search for, or "no matches".
 */
export function resultsView(
  state: Pick<SearchState, "query" | "hits" | "error" | "tookMs">,
  filters: { tags: string[] },
): "list" | "error" | "empty" | "none" {
  if (state.error !== null) {
    return "error";
  }
  if (!canSearch(state.query, filters)) {
    return "empty";
  }
  return state.hits.length === 0 && state.tookMs !== null ? "none" : "list";
}

/**
 * Wraps the model search so only the latest call's answer is used: typing or a filter
 * change fires a search, and an older, slower answer must never replace a newer one.
 * Nothing to search for clears the results without asking Rust.
 */
export function latestOnly(
  ask: (query: string, filters: Filters) => Promise<Answer | null> = searchNames,
  now: () => number = () => performance.now(),
): (query: string, filters: Filters) => Promise<SearchUpdate | null> {
  let latest = 0;
  return async (query, filters) => {
    const mine = ++latest;
    if (!canSearch(query, filters)) {
      return CLEARED;
    }
    const start = now();
    try {
      let answer = await ask(query, filters);
      // Rust answers null when it cancelled this search for one that reached it
      // later. Calls can reach Rust out of order, so an older keystroke may have
      // cancelled this, the newest: ask once more, and it then wins.
      if (answer === null && mine === latest) {
        answer = await ask(query, filters);
      }
      if (mine !== latest || answer === null) {
        return null;
      }
      return {
        hits: answer.hits,
        selected: 0,
        error: null,
        tookMs: now() - start,
        partial: answer.partial,
      };
    } catch (err) {
      return mine === latest ? { ...CLEARED, error: String(err) } : null;
    }
  };
}

/**
 * The line above the results: how many, and how fast. A full page (`limit` results)
 * means there are more matches. When a date was checked on only the best name
 * matches, it says how to see more.
 */
export function describeResults(
  count: number,
  tookMs: number,
  { partial = false, limit = RESULTS_DEFAULT }: { partial?: boolean; limit?: number } = {},
): string {
  const amount =
    count >= limit
      ? `Top ${limit.toLocaleString("en-US")} results`
      : `${count.toLocaleString("en-US")} result${count === 1 ? "" : "s"}`;
  const more = partial ? " · type more of the name to find older matches" : "";
  return `${amount} · ${Math.max(1, Math.round(tookMs))} ms${more}`;
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
 * show why a result matched. Filters (`ext:`, `type:`, `tag:`) mark nothing.
 */
export function highlight(name: string, query: string): Segment[] {
  const lower = name.toLowerCase();
  // Folding can change the length of some characters; then offsets would not line up.
  if (lower.length !== name.length) {
    return [{ text: name, match: false }];
  }
  const marks = new Array<boolean>(name.length).fill(false);
  for (const token of query.toLowerCase().split(/\s+/)) {
    if (token === "" || /^(ext|type|tag):/.test(token)) {
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
