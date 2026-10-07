import { getCurrentWindow } from "@tauri-apps/api/window";

/** Window controls for the custom title bar; each needs its core:window permission. */
export function minimizeWindow(): Promise<void> {
  return getCurrentWindow().minimize();
}

export function toggleMaximizeWindow(): Promise<void> {
  return getCurrentWindow().toggleMaximize();
}

export function closeWindow(): Promise<void> {
  return getCurrentWindow().close();
}
