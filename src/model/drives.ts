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

/** The drive being indexed and searched; null only when no drive exists. */
export function getActiveDrive(): Promise<string | null> {
  return invoke<string | null>("get_active_drive");
}

/** Makes `letter` the one indexed drive; Rust validates it and returns what it stored. */
export function setActiveDrive(letter: string): Promise<string> {
  return invoke<string>("set_active_drive", { letter });
}
