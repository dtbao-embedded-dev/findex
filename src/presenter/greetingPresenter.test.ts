import { describe, expect, it } from "vitest";

import { submitName } from "./greetingPresenter";

describe("submitName", () => {
  it("shows the model's greeting", async () => {
    const state = await submitName("Ada", async (name) => `Hello, ${name}!`);
    expect(state).toEqual({ message: "Hello, Ada!", error: null });
  });

  it("refuses a blank name without calling the model", async () => {
    let called = false;
    const state = await submitName("   ", async () => {
      called = true;
      return "";
    });
    expect(state.error).toBe("Enter a name first.");
    expect(called).toBe(false);
  });

  it("shows the model's refusal as the error", async () => {
    const state = await submitName("x", async () => {
      throw "a name is required";
    });
    expect(state).toEqual({ message: "", error: "a name is required" });
  });
});
