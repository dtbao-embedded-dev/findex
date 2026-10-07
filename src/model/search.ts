import { invoke } from "@tauri-apps/api/core";

/** One search result, as `model/search.rs` returns it. */
export type Hit = {
  name: string;
  path: string;
  isDir: boolean;
  /** For a file found by its contents, the line of the first match, from 1. */
  line?: number;
  /** The text of that line around the match. */
  snippet?: string;
};

export type Kind = "any" | "file" | "folder";

/** What the filter bar narrows by, as `Filters` in `model/search.rs` reads it. */
export type Filters = {
  kind: Kind;
  tags: string[];
  /** File types to keep, by extension without the dot (`c`, `h`); any of them matches. */
  exts: string[];
  /** Last changed at or after, in ms since the epoch. */
  modifiedFrom: number | null;
  /** Last changed before, in ms since the epoch. */
  modifiedTo: number | null;
};

/** One page of a search, as `Answer` in `model/find.rs` returns it. */
export type Answer = {
  hits: Hit[];
  /** Every result there is; null when a date filter stopped before the last one. */
  total: number | null;
  /** Another page follows this one. */
  more: boolean;
  /** A date was checked on only the best name matches: more may match. */
  partial: boolean;
  /** Bytes of memory the search index holds. */
  indexBytes: number;
};

/**
 * Asks Rust for `count` results from rank `offset`; null when a newer search replaced
 * this one there.
 */
export function searchNames(
  query: string,
  filters: Filters,
  offset: number,
  count: number,
): Promise<Answer | null> {
  return invoke<Answer | null>("search_names", { query, filters, offset, count });
}

/** Stops the search running in Rust, such as a content scan nobody waits for. */
export function cancelSearch(): Promise<void> {
  return invoke<void>("cancel_search");
}

/** Shows the path selected in File Explorer. */
export function revealPath(path: string): Promise<void> {
  return invoke<void>("reveal_path", { path });
}
