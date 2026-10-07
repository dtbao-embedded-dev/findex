import { getActiveDrive, listDrives, setActiveDrive, type Drive } from "../model/drives";

/** The drive list the picker shows, or why it could not be read. */
export type DriveList = { drives: Drive[]; error: string | null };

/** One drive as the picker words it. */
export type DriveRow = {
  title: string;
  detail: string;
  method: string;
  usedPercent: number;
};

/**
 * The drive the search covers. Rust picks the system drive on the first launch, so
 * this is null only when no drive exists or the setting cannot be read.
 */
export async function loadActive(
  read: () => Promise<string | null> = getActiveDrive,
): Promise<string | null> {
  try {
    return await read();
  } catch {
    return null;
  }
}

/** Lists the drives; read each time the picker opens, so a drive plugged in shows up. */
export async function loadDrives(list: () => Promise<Drive[]> = listDrives): Promise<DriveList> {
  try {
    return { drives: await list(), error: null };
  } catch (err) {
    return { drives: [], error: String(err) };
  }
}

/**
 * Makes `letter` the active drive; Rust saves it and indexes that drive alone. On
 * failure the previous drive stays active and the error is shown.
 */
export async function chooseDrive(
  active: string | null,
  letter: string,
  save: (letter: string) => Promise<string> = setActiveDrive,
): Promise<{ active: string | null; error: string | null }> {
  if (letter === active) {
    return { active, error: null };
  }
  try {
    return { active: await save(letter), error: null };
  } catch (err) {
    return { active, error: String(err) };
  }
}

/** The picker button's label: the drive's Explorer name, or its letter if not listed. */
export function activeTitle(drives: Drive[], active: string | null): string {
  if (active === null) {
    return "Choose a drive";
  }
  const drive = drives.find((d) => d.letter === active);
  return drive ? describeDrive(drive).title : `${active}:`;
}

export function searchPlaceholder(active: string | null): string {
  return active === null ? "Search files and folders" : `Search ${active}:`;
}

const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

/** Sizes as Explorer shows them: powers of 1024, one decimal below 10. */
export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit > 0 && value < 10 ? 1 : 0;
  return `${value.toFixed(digits)} ${UNITS[unit]}`;
}

export function describeDrive(drive: Drive): DriveRow {
  const fallback = drive.removable ? "USB Drive" : "Local Disk";
  const used = drive.total > 0 ? (drive.total - drive.free) / drive.total : 0;
  return {
    title: `${drive.label || fallback} (${drive.letter}:)`,
    detail:
      drive.total > 0
        ? `${formatBytes(drive.free)} free of ${formatBytes(drive.total)}`
        : drive.filesystem,
    method:
      drive.method === "mft"
        ? `${drive.filesystem} · instant, stays current`
        : `${drive.filesystem} · folder scan`,
    usedPercent: Math.round(used * 100),
  };
}
