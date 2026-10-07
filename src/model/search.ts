import { invoke } from "@tauri-apps/api/core";

/** One search result, as `model/search.rs` returns it. */
export type Hit = {
  name: string;
  path: string;
  isDir: boolean;
};

export type Kind = "any" | "file" | "folder";

/** What the filter bar narrows by, as `Filters` in `model/search.rs` reads it. */
export type Filters = {
  kind: Kind;
  tags: string[];
  /** Last changed at or after, in ms since the epoch. */
  modifiedFrom: number | null;
  /** Last changed before, in ms since the epoch. */
  modifiedTo: number | null;
};

/** One search's answer, as `Answer` in `model/find.rs` returns it. */
export type Answer = {
  hits: Hit[];
  /** A date was checked on only the best name matches: more may match. */
  partial: boolean;
};

/** Runs a search in Rust; null when a newer search replaced this one there. */
export function searchNames(query: string, filters: Filters): Promise<Answer | null> {
  return invoke<Answer | null>("search_names", { query, filters });
}

/** Shows the path selected in File Explorer. */
export function revealPath(path: string): Promise<void> {
  return invoke<void>("reveal_path", { path });
}
