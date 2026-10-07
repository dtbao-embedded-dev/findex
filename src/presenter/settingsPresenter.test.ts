import { describe, expect, it } from "vitest";

import { loadLimit, parseLimit, RESULTS_DEFAULT, saveLimit } from "./settingsPresenter";

describe("parseLimit", () => {
  it("reads a whole number, with or without thousands separators", () => {
    expect(parseLimit("5000")).toEqual({ limit: 5000, error: null });
    expect(parseLimit(" 10,000 ")).toEqual({ limit: 10000, error: null });
    expect(parseLimit("200")).toEqual({ limit: 200, error: null });
  });

  it("refuses what is not a whole number or is out of range", () => {
    for (const text of ["", "abc", "12.5", "-300"]) {
      expect(parseLimit(text)).toEqual({ limit: null, error: "Type a whole number." });
    }
    for (const text of ["199", "10001", "0"]) {
      expect(parseLimit(text)).toEqual({
        limit: null,
        error: "Choose between 200 and 10,000 results.",
      });
    }
  });
});

describe("loadLimit", () => {
  it("reads the saved limit, or the default when it cannot", async () => {
    expect(await loadLimit(async () => 3000)).toBe(3000);
    expect(
      await loadLimit(async () => {
        throw "no ipc";
      }),
    ).toBe(RESULTS_DEFAULT);
    expect(RESULTS_DEFAULT).toBe(200);
  });
});

describe("saveLimit", () => {
  it("saves a valid limit and returns what Rust stored", async () => {
    let saved = 0;
    const result = await saveLimit("1,000", async (limit) => {
      saved = limit;
      return limit;
    });
    expect(result).toEqual({ limit: 1000, error: null });
    expect(saved).toBe(1000);
  });

  it("does not ask Rust for an invalid entry, and shows Rust's refusal", async () => {
    let asked = false;
    expect(
      await saveLimit("50", async (limit) => {
        asked = true;
        return limit;
      }),
    ).toEqual({ limit: null, error: "Choose between 200 and 10,000 results." });
    expect(asked).toBe(false);
    expect(await saveLimit("300", async () => Promise.reject("database is locked"))).toEqual({
      limit: null,
      error: "database is locked",
    });
  });
});
