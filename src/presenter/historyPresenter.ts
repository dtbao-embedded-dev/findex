import { getSetting, setSetting } from "../model/settings";

/** The setting the history is saved under, as a JSON list, newest first. */
export const HISTORY_KEY = "search.history";

/** How many recent searches are kept. */
export const HISTORY_MAX = 20;

/** A query that stays on screen this long, with results, is worth remembering. */
export const REMEMBER_AFTER_MS = 1500;

/** Shorter than this is a keystroke on the way, not a search. */
const MIN_LENGTH = 2;

/**
 * The history with `query` at the top: the same query in another case moves up
 * rather than repeating, and when typing went on from the newest entry ("rea", then
 * "readme"), the longer one replaces it. Returns the same list when nothing
 * changes, so the caller knows not to save.
 */
export function remember(history: string[], query: string): string[] {
  const text = query.trim();
  if (text.length < MIN_LENGTH || history[0] === text) {
    return history;
  }
  const lower = text.toLowerCase();
  const newest = history[0]?.toLowerCase();
  const rest = history.filter(
    (entry, index) =>
      entry.toLowerCase() !== lower && !(index === 0 && newest !== undefined && lower.startsWith(newest)),
  );
  return [text, ...rest].slice(0, HISTORY_MAX);
}

/** The history without `query`. */
export function forget(history: string[], query: string): string[] {
  return history.filter((entry) => entry !== query);
}

/** The history as saved; anything unreadable counts as empty. */
export function parseHistory(text: string | null): string[] {
  try {
    const value: unknown = JSON.parse(text ?? "[]");
    return Array.isArray(value)
      ? value
          .filter((entry): entry is string => typeof entry === "string" && entry.trim() !== "")
          .slice(0, HISTORY_MAX)
      : [];
  } catch {
    return [];
  }
}

/** Reads the saved history; a failed read only means no history this time. */
export async function loadHistory(
  read: (key: string) => Promise<string | null> = getSetting,
): Promise<string[]> {
  try {
    return parseHistory(await read(HISTORY_KEY));
  } catch {
    return [];
  }
}

/** Saves the history; a failed write is not worth interrupting a search for. */
export async function saveHistory(
  history: string[],
  write: (key: string, value: string) => Promise<void> = setSetting,
): Promise<void> {
  try {
    await write(HISTORY_KEY, JSON.stringify(history));
  } catch {
    // Kept in memory for this session regardless.
  }
}
