import { invoke } from "@tauri-apps/api/core";

/** (Re)indexes the drives saved in setup; Rust reports progress as events. */
export function startIndex(): Promise<void> {
  return invoke<void>("start_index");
}
