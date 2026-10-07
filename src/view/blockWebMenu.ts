/** The element properties that decide whether the WebView's own menu may open. */
export type MenuTarget = {
  tagName: string;
  isContentEditable: boolean;
  type?: string;
};

/** Input types that hold editable text, where cut, copy and paste are worth keeping. */
const TEXT_INPUTS = new Set(["text", "search", "url", "email", "tel", "password", "number"]);

/** True where the WebView's right-click menu stays: text fields only. */
export function allowsNativeMenu(target: MenuTarget | null): boolean {
  if (!target) {
    return false;
  }
  if (target.isContentEditable || target.tagName === "TEXTAREA") {
    return true;
  }
  return target.tagName === "INPUT" && TEXT_INPUTS.has((target.type ?? "text").toLowerCase());
}

/**
 * Stops the browser menu (Back, Reload, Inspect…) from opening on right-click: it
 * belongs to a web page, not to a desktop app. A component that offers its own menu
 * handles `contextmenu` before this listener on `document` sees it.
 */
export function blockWebMenu(): void {
  document.addEventListener("contextmenu", (event) => {
    const target = event.target instanceof HTMLElement ? event.target : null;
    if (!allowsNativeMenu(target)) {
      event.preventDefault();
    }
  });
}
