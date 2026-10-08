import { getSetting, setSetting } from "../model/settings";

/** The setting the chosen theme is saved under. */
export const THEME_KEY = "appearance.theme";

/** Findex is dark only; these are its themes, each defined in `styles.css`. */
export const THEMES = [
  { id: "graphite", name: "Graphite" },
  { id: "midnight", name: "Midnight" },
  { id: "aurora", name: "Aurora" },
  { id: "violet", name: "Violet" },
  { id: "forest", name: "Forest" },
  { id: "ember", name: "Ember" },
] as const;

export type ThemeId = (typeof THEMES)[number]["id"];

export const DEFAULT_THEME: ThemeId = "graphite";

/** A saved value as a theme; anything unknown (or the light theme of old) is Graphite. */
export function toTheme(value: string | null): ThemeId {
  return THEMES.find((t) => t.id === value)?.id ?? DEFAULT_THEME;
}

/** The saved theme; a failed read shows the default rather than nothing. */
export async function loadTheme(
  read: (key: string) => Promise<string | null> = getSetting,
): Promise<ThemeId> {
  try {
    return toTheme(await read(THEME_KEY));
  } catch {
    return DEFAULT_THEME;
  }
}

/** Saves the theme; the error message, or null. */
export async function saveTheme(
  theme: ThemeId,
  write: (key: string, value: string) => Promise<void> = setSetting,
): Promise<string | null> {
  try {
    await write(THEME_KEY, theme);
    return null;
  } catch (err) {
    return String(err);
  }
}
