import { useEffect, useRef, useState, type ReactNode } from "react";

import {
  RESULTS_MAX,
  RESULTS_MIN,
  RESULTS_PRESETS,
  parseLimit,
  saveLimit,
} from "../presenter/settingsPresenter";

type Props = {
  /** How many results a search shows now. */
  limit: number;
  onLimitChange: (limit: number) => void;
  onClose: () => void;
  /** Extra sections from other features, in order. */
  children?: ReactNode;
};

/**
 * The settings, as a sheet that drops from under the title bar over a dimmed
 * window: a modal task, so the rest waits. Esc, the close button or a press on the
 * dimmed area closes it; Tab stays inside it. Each change is saved as it is made.
 */
export function SettingsSheet({ limit, onLimitChange, onClose, children }: Props) {
  const sheet = useRef<HTMLDivElement>(null);
  const [draft, setDraft] = useState(String(limit));
  const [error, setError] = useState<string | null>(null);

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

  const commit = (text: string) => {
    void saveLimit(text).then((entry) => {
      setError(entry.error);
      if (entry.limit !== null) {
        setDraft(String(entry.limit));
        onLimitChange(entry.limit);
      }
    });
  };

  // The slider follows what is typed while it is valid, and the saved value otherwise.
  const typed = parseLimit(draft).limit;

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

        <section className="sheet-section" aria-labelledby="settings-search">
          <h3 id="settings-search" className="sheet-heading">
            Search
          </h3>
          <div className="setting-group">
            <div className="setting-row">
              <div className="setting-text">
                <label htmlFor="results-limit">Results per search</label>
                <p className="setting-hint">
                  {RESULTS_MIN.toLocaleString("en-US")} to {RESULTS_MAX.toLocaleString("en-US")}.
                  More results take a little longer to show.
                </p>
              </div>
              <input
                id="results-limit"
                className="setting-number"
                type="text"
                inputMode="numeric"
                value={draft}
                aria-invalid={error !== null}
                aria-describedby={error !== null ? "results-limit-error" : undefined}
                onChange={(event) => setDraft(event.target.value)}
                onBlur={() => draft !== String(limit) && commit(draft)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    commit(draft);
                  }
                }}
              />
            </div>
            <div className="setting-row is-stacked">
              <input
                className="setting-slider"
                type="range"
                min={RESULTS_MIN}
                max={RESULTS_MAX}
                step={100}
                value={typed ?? limit}
                aria-label="Results per search"
                onChange={(event) => setDraft(event.target.value)}
                onPointerUp={(event) => commit(event.currentTarget.value)}
                onKeyUp={(event) => commit(event.currentTarget.value)}
              />
              <div className="setting-presets" role="group" aria-label="Common choices">
                {RESULTS_PRESETS.map((preset) => (
                  <button
                    key={preset}
                    type="button"
                    className="setting-chip"
                    aria-pressed={limit === preset}
                    onClick={() => commit(String(preset))}
                  >
                    {preset.toLocaleString("en-US")}
                  </button>
                ))}
              </div>
              {error !== null && (
                <p id="results-limit-error" className="error setting-error" role="alert">
                  {error}
                </p>
              )}
            </div>
          </div>
        </section>

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
