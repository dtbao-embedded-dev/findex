import { invoke } from "@tauri-apps/api/core";

/** Reads one stored setting; null when it was never set. */
export function getSetting(key: string): Promise<string | null> {
  return invoke<string | null>("get_setting", { key });
}

/** Stores one setting, replacing any earlier value. */
export function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>("set_setting", { key, value });
}

/** How many results a search shows, as `model/settings.rs` reads it. */
export function getResultsLimit(): Promise<number> {
  return invoke<number>("get_results_limit");
}

/** Saves how many results a search shows; Rust refuses a value out of range. */
export function setResultsLimit(limit: number): Promise<number> {
  return invoke<number>("set_results_limit", { limit });
}
