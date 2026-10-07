import { getExplorerMenu, setExplorerMenu, syncExplorerMenu } from "../model/explorer";
import type { TagInfo } from "../model/tags";

/** The settings switch: on or off, and why the last change failed if it did. */
export type MenuSetting = { enabled: boolean; error: string | null };

/** Reads the switch; the menu is on by default, so a failed read shows it on. */
export async function loadExplorerMenu(
  read: () => Promise<boolean> = getExplorerMenu,
): Promise<MenuSetting> {
  try {
    return { enabled: await read(), error: null };
  } catch (err) {
    return { enabled: true, error: String(err) };
  }
}

/** Turns the menu on or off; on failure the switch stays where it was. */
export async function setExplorerMenuTo(
  enabled: boolean,
  current: boolean,
  write: (enabled: boolean) => Promise<void> = setExplorerMenu,
): Promise<MenuSetting> {
  try {
    await write(enabled);
    return { enabled, error: null };
  } catch (err) {
    return { enabled: current, error: String(err) };
  }
}

/**
 * The tag names the menu lists, as one string: it changes when a tag is made,
 * deleted or added to a folder for the first time, not when a folder count moves,
 * so the registry is rewritten only when the menu would differ.
 */
export function menuTagsKey(catalog: TagInfo[]): string {
  return catalog.map((t) => t.tag).join("\n");
}

/** Rewrites the menu for the current tags; the error message, or null. */
export async function syncMenu(sync: () => Promise<void> = syncExplorerMenu): Promise<string | null> {
  try {
    await sync();
    return null;
  } catch (err) {
    return String(err);
  }
}
