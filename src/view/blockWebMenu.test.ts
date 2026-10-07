import { describe, expect, it } from "vitest";

import { allowsNativeMenu, type MenuTarget } from "./blockWebMenu";

/** The few element properties the predicate reads, without a DOM. */
function element(tagName: string, more: Partial<MenuTarget> = {}): MenuTarget {
  return { tagName, isContentEditable: false, ...more };
}

describe("allowsNativeMenu", () => {
  it("keeps the native menu in text fields, for cut, copy and paste", () => {
    expect(allowsNativeMenu(element("INPUT", { type: "text" }))).toBe(true);
    expect(allowsNativeMenu(element("INPUT", { type: "search" }))).toBe(true);
    expect(allowsNativeMenu(element("TEXTAREA"))).toBe(true);
    expect(allowsNativeMenu(element("DIV", { isContentEditable: true }))).toBe(true);
  });

  it("blocks it everywhere else, including inputs that hold no text", () => {
    expect(allowsNativeMenu(element("BODY"))).toBe(false);
    expect(allowsNativeMenu(element("LI"))).toBe(false);
    expect(allowsNativeMenu(element("BUTTON"))).toBe(false);
    expect(allowsNativeMenu(element("INPUT", { type: "checkbox" }))).toBe(false);
  });

  it("blocks it when the target is not an element", () => {
    expect(allowsNativeMenu(null)).toBe(false);
  });
});
