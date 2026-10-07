import { describe, expect, it } from "vitest";

import { initialSplash, loadSplash, revealMain, whenReady } from "./splashPresenter";

describe("loadSplash", () => {
  it("shows the name, version and copyright from the model", async () => {
    const state = await loadSplash(async () => ({
      name: "Findex",
      version: "0.1.0",
      copyright: "© 2026 dtbao",
    }));
    expect(state).toEqual({
      name: "Findex",
      version: "Version 0.1.0",
      copyright: "© 2026 dtbao",
    });
  });

  it("falls back to the name alone when the model fails", async () => {
    const state = await loadSplash(async () => {
      throw "no ipc";
    });
    expect(state).toEqual(initialSplash);
  });
});

describe("whenReady", () => {
  it("waits for both the work and the minimum time", async () => {
    const order: string[] = [];
    let releaseWait: () => void = () => {};
    const wait = () =>
      new Promise<void>((resolve) => {
        releaseWait = () => {
          order.push("min time");
          resolve();
        };
      });
    const ready = whenReady(
      Promise.resolve().then(() => order.push("work")),
      900,
      wait,
    ).then(() => order.push("ready"));
    await Promise.resolve();
    await Promise.resolve();
    expect(order).toEqual(["work"]);
    releaseWait();
    await ready;
    expect(order).toEqual(["work", "min time", "ready"]);
  });

  it("still resolves when the work fails", async () => {
    await expect(
      whenReady(Promise.reject(new Error("boom")), 0, async () => {}),
    ).resolves.toBeUndefined();
  });
});

describe("revealMain", () => {
  it("finishes the splash after the work", async () => {
    const order: string[] = [];
    await revealMain(
      Promise.resolve().then(() => order.push("work")),
      async () => {
        order.push("finish");
      },
      0,
      async () => {},
    );
    expect(order).toEqual(["work", "finish"]);
  });

  it("swallows a failed finish", async () => {
    await expect(
      revealMain(Promise.resolve(), async () => Promise.reject("gone"), 0, async () => {}),
    ).resolves.toBeUndefined();
  });
});
