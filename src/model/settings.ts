import { invoke } from "@tauri-apps/api/core";

/** Reads one stored setting; null when it was never set. */
export function getSetting(key: string): Promise<string | null> {
  return invoke<string | null>("get_setting", { key });
}

/** Stores one setting, replacing any earlier value. */
export function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>("set_setting", { key, value });
}
