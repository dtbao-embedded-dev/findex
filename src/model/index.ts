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

export function getIndexStatus(): Promise<IndexStatus> {
  return invoke<IndexStatus>("index_status");
}

/** Reads the active drive again from scratch; resolves to its letter once started. */
export function rescanDrive(): Promise<string> {
  return invoke<string>("rescan_drive");
}

/** Calls `onChange` with every status Rust sends; resolves to the unsubscribe call. */
export function onIndexStatus(onChange: (status: IndexStatus) => void): Promise<UnlistenFn> {
  return listen<IndexStatus>("index-status", (event) => onChange(event.payload));
}
