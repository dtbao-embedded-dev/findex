import { getScanDrives, listDrives, setScanDrives, type Drive } from "../model/drives";
import { startIndex } from "../model/index";

/** What the drive setup screen shows. */
export type SetupState = {
  drives: Drive[];
  selected: string[];
  saving: boolean;
  error: string | null;
};

export const initialSetup: SetupState = { drives: [], selected: [], saving: false, error: null };

/** One drive as the setup list words it. */
export type DriveRow = {
  title: string;
  detail: string;
  method: string;
  usedPercent: number;
};

/** Setup is needed until a drive list has been saved once; an unreadable one asks again. */
export async function needsSetup(
  saved: () => Promise<string[] | null> = getScanDrives,
): Promise<boolean> {
  try {
    return (await saved()) === null;
  } catch {
    return true;
  }
}

/** The system drive when present, otherwise the first fast-indexable one. */
export function defaultSelection(drives: Drive[]): string[] {
  const pick =
    drives.find((d) => d.letter === "C") ??
    drives.find((d) => d.method === "mft" && !d.removable) ??
    drives[0];
  return pick ? [pick.letter] : [];
}

/**
 * Lists the drives with the saved choice ticked, or the default on first run. A
 * saved drive that is not plugged in now is dropped: it could not be unticked.
 */
export async function loadSetup(
  list: () => Promise<Drive[]> = listDrives,
  saved: () => Promise<string[] | null> = getScanDrives,
): Promise<SetupState> {
  try {
    const [drives, chosen] = await Promise.all([list(), saved().catch(() => null)]);
    const present = new Set(drives.map((d) => d.letter));
    const selected =
      chosen === null ? defaultSelection(drives) : chosen.filter((l) => present.has(l));
    return { ...initialSetup, drives, selected };
  } catch (err) {
    return { ...initialSetup, error: String(err) };
  }
}

export function toggleDrive(state: SetupState, letter: string): SetupState {
  const selected = state.selected.includes(letter)
    ? state.selected.filter((l) => l !== letter)
    : [...state.selected, letter].sort();
  return { ...state, selected, error: null };
}

export function canContinue(state: SetupState): boolean {
  return state.selected.length > 0 && !state.saving;
}

/**
 * Saves the choice and reindexes with it. `saved` is true once both happened; if
 * indexing fails to start, the error keeps the screen open to retry.
 */
export async function saveSetup(
  state: SetupState,
  save: (letters: string[]) => Promise<string[]> = setScanDrives,
  reindex: () => Promise<void> = startIndex,
): Promise<{ state: SetupState; saved: boolean }> {
  if (state.selected.length === 0) {
    return { state: { ...state, error: "Choose at least one drive." }, saved: false };
  }
  let selected: string[];
  try {
    selected = await save(state.selected);
  } catch (err) {
    return { state: { ...state, saving: false, error: String(err) }, saved: false };
  }
  try {
    await reindex();
    return { state: { ...state, selected, saving: false, error: null }, saved: true };
  } catch (err) {
    return { state: { ...state, saving: false, error: String(err) }, saved: false };
  }
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
