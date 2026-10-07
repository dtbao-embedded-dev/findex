import { invoke } from "@tauri-apps/api/core";

/** One local drive; `method` is how its names are read (see `model/drives.rs`). */
export type Drive = {
  letter: string;
  label: string;
  filesystem: string;
  removable: boolean;
  total: number;
  free: number;
  method: "mft" | "walk";
};

export function listDrives(): Promise<Drive[]> {
  return invoke<Drive[]>("list_drives");
}

/** The drive letters chosen in setup, or null before setup has been done. */
export function getScanDrives(): Promise<string[] | null> {
  return invoke<string[] | null>("get_scan_drives");
}

/** Stores the chosen letters; Rust validates them and returns what it stored. */
export function setScanDrives(letters: string[]): Promise<string[]> {
  return invoke<string[]>("set_scan_drives", { letters });
}
