import { useEffect, useRef, type ReactNode } from "react";

type Props = {
  onClose: () => void;
  /** The sections of each feature, in order. */
  children?: ReactNode;
};

/**
 * The settings, as a sheet that drops from under the title bar over a dimmed
 * window: a modal task, so the rest waits. Esc, the close button or a press on the
 * dimmed area closes it; Tab stays inside it. Each change is saved as it is made.
 */
export function SettingsSheet({ onClose, children }: Props) {
  const sheet = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    sheet.current?.querySelector<HTMLElement>("input, button")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onClose();
      } else if (event.key === "Tab") {
        trapTab(event, sheet.current);
      }
    };
    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      previous?.focus();
    };
  }, [onClose]);

  return (
    <div
      className="sheet-scrim"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <div
        ref={sheet}
        className="sheet"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        <header className="sheet-header">
          <h2 id="settings-title">Settings</h2>
          <button type="button" className="sheet-close" aria-label="Close settings" onClick={onClose}>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
            </svg>
          </button>
        </header>

        {children}
      </div>
    </div>
  );
}

/** Keeps Tab and Shift+Tab cycling inside the sheet. */
function trapTab(event: KeyboardEvent, sheet: HTMLElement | null) {
  const focusable = Array.from(
    sheet?.querySelectorAll<HTMLElement>("button, input, [tabindex]:not([tabindex='-1'])") ?? [],
  ).filter((el) => !el.hasAttribute("disabled"));
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (!first || !last) {
    return;
  }
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}
