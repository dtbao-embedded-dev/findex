import { invoke } from "@tauri-apps/api/core";

/** One result's details, as `model/details.rs` reads them from the disk. */
export type Details = {
  path: string;
  /** Explorer's name for the type, such as "PDF Document". */
  kind: string;
  /** Bytes, for files only. */
  size: number | null;
  /** Last change, in milliseconds since the Unix epoch. */
  modified: number | null;
  /** The folder's tags, sorted; files carry none. */
  tags: string[];
};

/** Details and tags for each path that still exists; deleted ones are left out. */
export function describePaths(paths: string[]): Promise<Details[]> {
  return invoke<Details[]>("describe_paths", { paths });
}
