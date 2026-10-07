import { describe, expect, it } from "vitest";

import { initialTitle, loadTitle } from "./aboutPresenter";

describe("loadTitle", () => {
  it("names the app and its version, prefixed with v", async () => {
    const title = await loadTitle(async () => ({
      name: "Findex",
      version: "0.1.0",
      copyright: "© 2026 dtbao",
    }));
    expect(title).toEqual({ name: "Findex", version: "v0.1.0" });
  });

  it("shows no version when the model has none", async () => {
    const title = await loadTitle(async () => ({ name: "Findex", version: "", copyright: "" }));
    expect(title).toEqual({ name: "Findex", version: "" });
  });

  it("falls back to the name alone when the model fails", async () => {
    const title = await loadTitle(async () => {
      throw "no ipc";
    });
    expect(title).toEqual(initialTitle);
    expect(initialTitle).toEqual({ name: "Findex", version: "" });
  });
});
