import { useEffect, type RefObject } from "react";

/**
 * Closes a floating panel on Esc, a press outside it, or the window losing focus or
 * size. Esc is caught before the search field sees it, so closing a panel never also
 * clears the query.
 */
export function useDismiss(panel: RefObject<HTMLElement | null>, onClose: () => void): void {
  useEffect(() => {
    const outside = (event: PointerEvent) => {
      if (!panel.current?.contains(event.target as Node)) {
        onClose();
      }
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    document.addEventListener("pointerdown", outside, true);
    document.addEventListener("keydown", escape, true);
    window.addEventListener("blur", onClose);
    window.addEventListener("resize", onClose);
    return () => {
      document.removeEventListener("pointerdown", outside, true);
      document.removeEventListener("keydown", escape, true);
      window.removeEventListener("blur", onClose);
      window.removeEventListener("resize", onClose);
    };
  }, [panel, onClose]);
}
