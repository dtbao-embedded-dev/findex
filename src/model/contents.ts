import { Channel, invoke } from "@tauri-apps/api/core";

import type { Filters, Hit } from "./search";

/** What a content search sends as it goes, as `Batch` in `model/contents.rs`. */
export type Batch = {
  /** Name matches first, then files found by a line inside them. */
  hits: Hit[];
  /** Files whose contents were read so far. */
  filesRead: number;
  /** Files to look at in all; 0 while the name matches are being sent. */
  filesTotal: number;
  /** The last batch of a scan that ran to its end. */
  done: boolean;
};

/**
 * Searches names and file contents in Rust, handing each batch to `onBatch` as it
 * arrives. Resolves once the scan ended or a newer search stopped it.
 */
export function searchContents(
  query: string,
  filters: Filters,
  onBatch: (batch: Batch) => void,
): Promise<void> {
  const channel = new Channel<Batch>();
  channel.onmessage = onBatch;
  return invoke<void>("search_contents", { query, filters, onBatch: channel });
}
