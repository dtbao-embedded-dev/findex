import { describe, expect, it } from "vitest";

import { DEFAULT_THEME, THEMES, loadTheme, saveTheme, toTheme } from "./themePresenter";

describe("themes", () => {
  it("offers Graphite and five others, Graphite first as the default", () => {
    expect(THEMES.map((t) => t.id)).toEqual([
      "graphite",
      "midnight",
      "aurora",
      "violet",
      "forest",
      "ember",
    ]);
    expect(DEFAULT_THEME).toBe("graphite");
  });

  it("reads a saved theme and falls back to Graphite for anything else", () => {
    expect(toTheme("ember")).toBe("ember");
    expect(toTheme("light")).toBe("graphite");
    expect(toTheme(null)).toBe("graphite");
  });

  it("loads and saves the setting, keeping the window usable when either fails", async () => {
    expect(await loadTheme(async () => "violet")).toBe("violet");
    expect(
      await loadTheme(async () => {
        throw "no database";
      }),
    ).toBe("graphite");
    const written: [string, string][] = [];
    expect(
      await saveTheme("forest", async (key, value) => {
        written.push([key, value]);
      }),
    ).toBeNull();
    expect(written).toEqual([["appearance.theme", "forest"]]);
    expect(
      await saveTheme("forest", async () => {
        throw "read-only";
      }),
    ).toBe("read-only");
  });
});
