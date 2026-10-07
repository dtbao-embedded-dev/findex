import { getResultsLimit, setResultsLimit } from "../model/settings";

/** The range `model/settings.rs` accepts for results per search. */
export const RESULTS_MIN = 200;
export const RESULTS_MAX = 10_000;
export const RESULTS_DEFAULT = 200;

/** One-press choices offered beside the field. */
export const RESULTS_PRESETS = [200, 1_000, 5_000, 10_000];

/** A typed limit, or why it cannot be used. */
export type LimitEntry = { limit: number | null; error: string | null };

const count = (n: number) => n.toLocaleString("en-US");

/** Reads what was typed as a limit; thousands separators are allowed. */
export function parseLimit(text: string): LimitEntry {
  const digits = text.trim().replace(/,/g, "");
  if (!/^\d+$/.test(digits)) {
    return { limit: null, error: "Type a whole number." };
  }
  const limit = Number(digits);
  if (limit < RESULTS_MIN || limit > RESULTS_MAX) {
    return {
      limit: null,
      error: `Choose between ${count(RESULTS_MIN)} and ${count(RESULTS_MAX)} results.`,
    };
  }
  return { limit, error: null };
}

/** The saved limit; the default when it cannot be read, since search still works. */
export async function loadLimit(read: () => Promise<number> = getResultsLimit): Promise<number> {
  try {
    return await read();
  } catch {
    return RESULTS_DEFAULT;
  }
}

/**
 * Saves what was typed. An invalid entry never reaches Rust; Rust checks again and
 * its refusal is shown as is.
 */
export async function saveLimit(
  text: string,
  save: (limit: number) => Promise<number> = setResultsLimit,
): Promise<LimitEntry> {
  const entry = parseLimit(text);
  if (entry.limit === null) {
    return entry;
  }
  try {
    return { limit: await save(entry.limit), error: null };
  } catch (err) {
    return { limit: null, error: String(err) };
  }
}
