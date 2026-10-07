import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

/**
 * Installs a newer signed release if the update endpoint offers one, then restarts
 * into it. Returns the version installed, or null when this one is current.
 * Call it from a presenter that has asked the user first.
 */
export async function installUpdate(): Promise<string | null> {
  const update = await check();
  if (!update) {
    return null;
  }
  await update.downloadAndInstall();
  await relaunch();
  return update.version;
}
