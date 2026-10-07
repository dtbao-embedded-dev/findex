import { invoke } from "@tauri-apps/api/core";

/** One search result, as `model/search.rs` returns it. */
export type Hit = {
  name: string;
  path: string;
  isDir: boolean;
};

/** Runs a name search in Rust; null when a newer search replaced this one there. */
export function searchNames(query: string, limit?: number): Promise<Hit[] | null> {
  return invoke<Hit[] | null>("search_names", { query, limit });
}

/** Shows the path selected in File Explorer. */
export function revealPath(path: string): Promise<void> {
  return invoke<void>("reveal_path", { path });
}
