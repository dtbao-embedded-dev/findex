import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

/** A newer signed release, already downloaded and waiting to be installed. */
export type Downloaded = {
  version: string;
  /** Runs the installer. On Windows the app exits for it and the installer restarts it. */
  install: () => Promise<void>;
};

/**
 * Asks the update endpoint for a newer signed release and downloads it. Resolves to
 * null when this version is current; rejects when the endpoint cannot be reached or
 * the download fails its signature check.
 */
export async function fetchUpdate(): Promise<Downloaded | null> {
  const update = await check();
  if (!update) {
    return null;
  }
  await update.download();
  return {
    version: update.version,
    install: async () => {
      await update.install();
      await relaunch();
    },
  };
}

/**
 * Saves the drive index changed by live updates. The installer ends the app without
 * the usual quit, which would otherwise save it.
 */
export function saveIndex(): Promise<void> {
  return invoke<void>("save_index");
}

/** Calls `onCheck` when the tray menu's "Check for updates" is chosen. */
export function onCheckRequested(onCheck: () => void): Promise<UnlistenFn> {
  return listen("check-updates", () => onCheck());
}
