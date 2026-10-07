import { useEffect, useRef, useState, type KeyboardEvent } from "react";

import {
  activeTitle,
  chooseDrive,
  describeDrive,
  driveLetterLabel,
  loadDrives,
  type DriveList,
} from "../presenter/drivePresenter";

type Props = {
  active: string | null;
  /** The drive now active, and why switching failed if it did. */
  onChange: (active: string | null, error: string | null) => void;
};

/**
 * The drive the search covers, as a compact button at the left of the search field:
 * its glyph and letter, with the full name and free space on hover. It opens a menu
 * of every drive; picking one indexes that drive alone. Esc or a click outside
 * closes it.
 */
export function DrivePicker({ active, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const [list, setList] = useState<DriveList>({ drives: [], error: null });
  const root = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);

  // The button names the drive before the menu is ever opened.
  useEffect(() => {
    void loadDrives().then(setList);
  }, []);

  useEffect(() => {
    if (!open) {
      return;
    }
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const escape = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        setOpen(false);
        button.current?.focus();
      }
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape, true);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape, true);
    };
  }, [open]);

  // Opening puts focus on the active drive, so arrows and Enter work at once.
  useEffect(() => {
    if (open) {
      const items = menu.current?.querySelectorAll<HTMLButtonElement>("[role=menuitemradio]");
      const checked = menu.current?.querySelector<HTMLButtonElement>("[aria-checked=true]");
      (checked ?? items?.[0])?.focus();
    }
  }, [open, list]);

  const toggle = () => {
    if (!open) {
      // Read again on every open: a drive plugged in since shows up.
      void loadDrives().then(setList);
    }
    setOpen((o) => !o);
  };

  const choose = (letter: string) => {
    setOpen(false);
    button.current?.focus();
    void chooseDrive(active, letter).then((result) => onChange(result.active, result.error));
  };

  const moveFocus = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = { ArrowDown: 1, ArrowUp: -1 }[event.key as "ArrowDown" | "ArrowUp"];
    const items = Array.from(
      menu.current?.querySelectorAll<HTMLButtonElement>("[role=menuitemradio]") ?? [],
    );
    if (step === undefined || items.length === 0) {
      return;
    }
    event.preventDefault();
    const at = items.indexOf(document.activeElement as HTMLButtonElement);
    items[(at + step + items.length) % items.length]?.focus();
  };

  const current = list.drives.find((d) => d.letter === active);

  return (
    <div className="drive-picker" ref={root}>
      <button
        ref={button}
        type="button"
        className="drive-button"
        aria-haspopup="menu"
        aria-expanded={open}
        title={
          current
            ? `${activeTitle(list.drives, active)} · ${describeDrive(current).detail}`
            : "The drive Findex indexes and searches"
        }
        aria-label={`Drive: ${activeTitle(list.drives, active)}`}
        onClick={toggle}
      >
        <DriveGlyph removable={current?.removable ?? false} />
        <span className="drive-button-label">{driveLetterLabel(active)}</span>
        <svg className="chevron" viewBox="0 0 12 12" aria-hidden="true">
          <path d="M3 4.5l3 3 3-3" fill="none" stroke="currentColor" strokeWidth="1.5" />
        </svg>
      </button>

      {open && (
        <div ref={menu} className="drive-menu" role="menu" aria-label="Drives" onKeyDown={moveFocus}>
          {list.error !== null && <p className="error drive-menu-note">{list.error}</p>}
          {list.drives.map((drive) => {
            const row = describeDrive(drive);
            const checked = drive.letter === active;
            return (
              <button
                key={drive.letter}
                type="button"
                role="menuitemradio"
                aria-checked={checked}
                className={checked ? "drive-option is-selected" : "drive-option"}
                onClick={() => choose(drive.letter)}
              >
                <DriveGlyph removable={drive.removable} />
                <span className="drive-text">
                  <span className="drive-title">{row.title}</span>
                  <span className="drive-detail">{row.detail}</span>
                  <span className="drive-meter" aria-hidden="true">
                    <span style={{ width: `${row.usedPercent}%` }} />
                  </span>
                </span>
                <span className="drive-badge" data-method={drive.method}>
                  {row.method}
                </span>
                <span className="drive-check" aria-hidden="true" />
              </button>
            );
          })}
          <p className="drive-menu-note">
            Findex indexes one drive at a time. A drive you leave keeps its saved index, so
            coming back to it is quick.
          </p>
        </div>
      )}
    </div>
  );
}

/** A drive, drawn rather than loaded, so it follows the text colour. */
function DriveGlyph({ removable }: { removable: boolean }) {
  return (
    <svg className="drive-glyph" viewBox="0 0 32 32" aria-hidden="true">
      {removable ? (
        <>
          <rect x="9" y="3" width="14" height="8" rx="1.5" fill="none" stroke="currentColor" strokeWidth="2" />
          <rect x="6" y="10" width="20" height="19" rx="4" fill="currentColor" opacity="0.18" stroke="currentColor" strokeWidth="2" />
        </>
      ) : (
        <>
          <rect x="3" y="8" width="26" height="16" rx="4" fill="currentColor" opacity="0.18" stroke="currentColor" strokeWidth="2" />
          <circle cx="23" cy="16" r="1.75" fill="currentColor" />
          <path d="M8 16h8" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </>
      )}
    </svg>
  );
}
