import { describePaths, type Details } from "../model/details";
import { searchNames, type Answer, type Filters, type Hit } from "../model/search";
import { formatBytes } from "./drivePresenter";

/** Rows asked for at a time; the next page is loaded as the list scrolls near the end. */
export const PAGE = 500;

/** How far a content search got. */
export type ScanProgress = { filesRead: number; filesTotal: number; done: boolean };

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
  /** Bytes of memory the search index held at the last answer. */
  indexBytes: number | null;
  /** Every result there is, loaded or not; null while unknown. */
  total: number | null;
  /** Another page can be loaded. */
  more: boolean;
  /** The content search on screen; null for a name search. */
  scan: ScanProgress | null;
};

export const initialSearch: SearchState = {
  query: "",
  hits: [],
  selected: 0,
  error: null,
  tookMs: null,
  partial: false,
  indexBytes: null,
  total: null,
  more: false,
  scan: null,
};

/** The part of the state one search answers; null when a later search superseded it. */
export type SearchUpdate = Pick<
  SearchState,
  "hits" | "selected" | "error" | "tookMs" | "partial" | "indexBytes" | "total" | "more" | "scan"
>;

/** The rows loaded once the next page arrived, and what is known of the rest. */
export type PageUpdate = Pick<SearchState, "hits" | "total" | "more" | "partial">;

export const CLEARED: SearchUpdate = {
  hits: [],
  selected: 0,
  error: null,
  tookMs: null,
  partial: false,
  indexBytes: null,
  total: null,
  more: false,
  scan: null,
};

type Ask = (
  query: string,
  filters: Filters,
  offset: number,
  count: number,
) => Promise<Answer | null>;

/** The filters that can stand in for a typed name. */
type Selecting = { tags: string[]; exts?: string[] };

/**
 * Whether there is anything to ask: the index holds names only, so a search needs a
 * name; a tag alone also works, listing the folders that carry it, and so does a file
 * type, listing every name of that type.
 */
export function canSearch(query: string, filters: Selecting): boolean {
  return query.trim() !== "" || filters.tags.length > 0 || (filters.exts?.length ?? 0) > 0;
}

/**
 * What the results area shows: the list (also while the first answer is on its way),
 * the error, the empty screen when there is nothing to search for, or "no matches".
 */
export function resultsView(
  state: Pick<SearchState, "query" | "hits" | "error" | "tookMs">,
  filters: Selecting,
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
 * Wraps the model search so only the latest search's answers are used: typing or a
 * filter change fires a search, and an older, slower answer must never replace a newer
 * one. `search` answers the first `count` rows (a page, or the rows already shown when
 * refreshing); `more` loads the page after `loaded` for the same search, and its
 * answer is dropped once a newer search started. Nothing to search for clears the
 * results without asking Rust.
 */
export function latestOnly(
  ask: Ask = searchNames,
  now: () => number = () => performance.now(),
): {
  search: (query: string, filters: Filters, count?: number) => Promise<SearchUpdate | null>;
  more: (query: string, filters: Filters, loaded: Hit[]) => Promise<PageUpdate | null>;
  stop: () => void;
} {
  let latest = 0;
  // The search whose rows are on screen; a page belongs to it only while it is latest.
  let answered = 0;
  const search = async (
    query: string,
    filters: Filters,
    count = PAGE,
  ): Promise<SearchUpdate | null> => {
    const mine = ++latest;
    if (!canSearch(query, filters)) {
      answered = mine;
      return CLEARED;
    }
    const start = now();
    try {
      let answer = await ask(query, filters, 0, count);
      // Rust answers null when it cancelled this search for one that reached it
      // later. Calls can reach Rust out of order, so an older keystroke may have
      // cancelled this, the newest: ask once more, and it then wins.
      if (answer === null && mine === latest) {
        answer = await ask(query, filters, 0, count);
      }
      if (mine !== latest || answer === null) {
        return null;
      }
      answered = mine;
      return {
        hits: answer.hits,
        selected: 0,
        error: null,
        tookMs: now() - start,
        partial: answer.partial,
        indexBytes: answer.indexBytes,
        total: answer.total,
        more: answer.more,
        scan: null,
      };
    } catch (err) {
      if (mine !== latest) {
        return null;
      }
      answered = mine;
      return { ...CLEARED, error: String(err) };
    }
  };
  const more = async (
    query: string,
    filters: Filters,
    loaded: Hit[],
  ): Promise<PageUpdate | null> => {
    const mine = latest;
    // A newer search is on its way: the rows on screen are not its, so no page of it
    // may be added after them.
    if (answered !== mine) {
      return null;
    }
    let answer: Answer | null;
    try {
      answer = await ask(query, filters, loaded.length, PAGE);
    } catch {
      // A page that fails leaves the rows already shown; scrolling asks again.
      return null;
    }
    if (mine !== latest || answer === null) {
      return null;
    }
    return {
      hits: appendPage(loaded, answer.hits),
      total: answer.total,
      more: answer.more,
      partial: answer.partial,
    };
  };
  // A content search took over: answers still on their way are dropped.
  const stop = () => {
    latest += 1;
  };
  return { search, more, stop };
}

/**
 * The rows loaded with the next page after them. A file indexed between two pages
 * can shift the ranking by a row, so a result already shown is not shown twice.
 */
export function appendPage(loaded: Hit[], page: Hit[]): Hit[] {
  const shown = new Set(loaded.map((hit) => hit.path));
  return [...loaded, ...page.filter((hit) => !shown.has(hit.path))];
}

/**
 * The line above the results: how many, how fast, and how much memory the index
 * searched holds. It counts every result, not only the rows loaded so far; when the
 * count is not known yet, `+` says more follow. When a date was checked on only the
 * best name matches, it says how to see more.
 */
export function describeResults(
  loaded: number,
  tookMs: number,
  {
    total = null,
    more = false,
    partial = false,
    indexBytes = null,
  }: {
    total?: number | null;
    more?: boolean;
    partial?: boolean;
    indexBytes?: number | null;
  } = {},
): string {
  const count = total ?? loaded;
  const plus = total === null && more ? "+" : "";
  const noun = count === 1 && plus === "" ? "result" : "results";
  const amount = `${count.toLocaleString("en-US")}${plus} ${noun}`;
  const memory = indexBytes === null ? "" : ` · ${formatBytes(indexBytes)} RAM`;
  const hint = partial ? " · type more of the name to find older matches" : "";
  return `${amount} · ${Math.max(1, Math.round(tookMs))} ms${memory}${hint}`;
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
