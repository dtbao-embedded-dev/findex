import { invoke } from "@tauri-apps/api/core";

/** Whether "Tag with Findex" is in Explorer's right-click menu (on unless turned off). */
export function getExplorerMenu(): Promise<boolean> {
  return invoke<boolean>("explorer_menu");
}

/** Turns the Explorer menu on or off; Rust saves the choice and edits the registry. */
export function setExplorerMenu(enabled: boolean): Promise<void> {
  return invoke<void>("set_explorer_menu", { enabled });
}

/** Rewrites the Explorer menu so it lists the tags as they are now. */
export function syncExplorerMenu(): Promise<void> {
  return invoke<void>("sync_explorer_menu");
}
