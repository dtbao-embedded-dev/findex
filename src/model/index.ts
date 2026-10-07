import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** One drive's indexing state, as `model/index/service.rs` reports it. */
export type DriveStatus = {
  letter: string;
  state: "indexing" | "ready" | "failed";
  /** Entries read so far by the running build; the full count once ready. */
  entries: number;
  /** Entries searchable right now: a loaded snapshot or the last finished build. */
  searchable: number;
  method: "mft" | "walk";
  note: string | null;
};

export type IndexStatus = {
  drives: DriveStatus[];
};

/** (Re)indexes the drives saved in setup; Rust reports progress as events. */
export function startIndex(): Promise<void> {
  return invoke<void>("start_index");
}

export function getIndexStatus(): Promise<IndexStatus> {
  return invoke<IndexStatus>("index_status");
}

/** Calls `onChange` with every status Rust sends; resolves to the unsubscribe call. */
export function onIndexStatus(onChange: (status: IndexStatus) => void): Promise<UnlistenFn> {
  return listen<IndexStatus>("index-status", (event) => onChange(event.payload));
}
