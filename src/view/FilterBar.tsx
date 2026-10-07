import { useCallback, useRef, useState, type ReactNode, type RefObject } from "react";

import type { Kind } from "../model/search";
import type { TagCount } from "../model/tags";
import {
  CUSTOM_DATES,
  DATE_PRESETS,
  describeDate,
  emptyCustom,
  initialFilters,
  isFiltered,
  narrowsDate,
  toggleTag,
  type DateChoice,
  type FilterState,
} from "../presenter/filterPresenter";
import { useDismiss } from "./useDismiss";

type Props = {
  filters: FilterState;
  /** The tags in use, offered by the tag menu. */
  tags: TagCount[];
  onChange: (filters: FilterState) => void;
};

const KINDS: { kind: Kind; label: string }[] = [
  { kind: "any", label: "All" },
  { kind: "file", label: "Files" },
  { kind: "folder", label: "Folders" },
];

/**
 * The filters under the search field: when a result last changed, whether it is a
 * file or a folder, and the tags a folder carries. Each change searches again at
 * once; nothing waits for an Apply button.
 */
export function FilterBar({ filters, tags, onChange }: Props) {
  const set = (change: Partial<FilterState>) => onChange({ ...filters, ...change });

  return (
    <div className="filter-bar" role="toolbar" aria-label="Filters">
      <Popover
        label={describeDate(filters.date)}
        active={narrowsDate(filters.date)}
        icon={<CalendarGlyph />}
        title="When it last changed"
      >
        <DateMenu choice={filters.date} onChoose={(date) => set({ date })} />
      </Popover>

      <div className="segmented" role="radiogroup" aria-label="Kind">
        {KINDS.map((k) => (
          <button
            key={k.kind}
            type="button"
            role="radio"
            aria-checked={filters.kind === k.kind}
            onClick={() => set({ kind: k.kind })}
          >
            {k.label}
          </button>
        ))}
      </div>

      <Popover
        label={filters.tags.length === 0 ? "Tags" : filters.tags.map((t) => `#${t}`).join(" ")}
        active={filters.tags.length > 0}
        icon={<TagGlyph />}
        title="Folders carrying every tag picked"
      >
        {tags.length === 0 ? (
          <p className="filter-note">No tags yet. Right-click a folder and choose Tags… to add one.</p>
        ) : (
          <div className="filter-options" role="group" aria-label="Tags">
            {tags.map((t) => (
              <button
                key={t.tag}
                type="button"
                role="menuitemcheckbox"
                aria-checked={filters.tags.includes(t.tag)}
                className="filter-option"
                onClick={() => set({ tags: toggleTag(filters.tags, t.tag) })}
              >
                <span className="filter-check" aria-hidden="true" />
                <span className="filter-option-label">#{t.tag}</span>
                <span className="sidebar-count">{t.folders}</span>
              </button>
            ))}
          </div>
        )}
      </Popover>

      {isFiltered(filters) && (
        <button type="button" className="filter-clear" onClick={() => onChange(initialFilters)}>
          Clear filters
        </button>
      )}
    </div>
  );
}

/** The date menu: presets first, then a custom day, month, year or range. */
function DateMenu({ choice, onChoose }: { choice: DateChoice; onChoose: (c: DateChoice) => void }) {
  const custom = CUSTOM_DATES.find((c) => c.kind === choice.kind);
  return (
    <>
      <div className="filter-options" role="group" aria-label="Date">
        {DATE_PRESETS.map((p) => (
          <button
            key={p.choice.kind}
            type="button"
            role="menuitemradio"
            aria-checked={choice.kind === p.choice.kind}
            className="filter-option"
            onClick={() => onChoose(p.choice)}
          >
            <span className="filter-check" aria-hidden="true" />
            <span className="filter-option-label">{p.label}</span>
          </button>
        ))}
      </div>
      <div className="filter-custom">
        <p className="filter-heading">Custom</p>
        <div className="segmented is-small" role="radiogroup" aria-label="Custom date">
          {CUSTOM_DATES.map((c) => (
            <button
              key={c.kind}
              type="button"
              role="radio"
              aria-checked={custom?.kind === c.kind}
              onClick={() => custom?.kind !== c.kind && onChoose(emptyCustom(c.kind))}
            >
              {c.label}
            </button>
          ))}
        </div>
        {choice.kind === "day" && (
          <input
            type="date"
            aria-label="Day"
            value={choice.date}
            onChange={(e) => onChoose({ kind: "day", date: e.target.value })}
          />
        )}
        {choice.kind === "month" && (
          <input
            type="month"
            aria-label="Month"
            value={choice.month}
            onChange={(e) => onChoose({ kind: "month", month: e.target.value })}
          />
        )}
        {choice.kind === "year" && (
          <input
            type="number"
            aria-label="Year"
            inputMode="numeric"
            min={1970}
            max={2100}
            placeholder={String(new Date().getFullYear())}
            value={choice.year}
            onChange={(e) => onChoose({ kind: "year", year: e.target.value })}
          />
        )}
        {choice.kind === "range" && (
          <div className="filter-range">
            <input
              type="date"
              aria-label="From"
              value={choice.from}
              onChange={(e) => onChoose({ ...choice, from: e.target.value })}
            />
            <span aria-hidden="true">–</span>
            <input
              type="date"
              aria-label="To"
              value={choice.to}
              onChange={(e) => onChoose({ ...choice, to: e.target.value })}
            />
          </div>
        )}
      </div>
    </>
  );
}

type PopoverProps = {
  label: string;
  active: boolean;
  icon: ReactNode;
  title: string;
  children: ReactNode;
};

/** A filter button and the panel it opens, which grows out of the button. */
function Popover({ label, active, icon, title, children }: PopoverProps) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const close = useCallback(() => {
    // Back to the button when focus was inside the panel, as after Esc; a press
    // elsewhere moves focus there on its own.
    const inside =
      root.current?.contains(document.activeElement) || document.activeElement === document.body;
    setOpen(false);
    if (inside) {
      button.current?.focus();
    }
  }, []);

  return (
    <div className="filter" ref={root}>
      <button
        ref={button}
        type="button"
        className={active ? "filter-button is-active" : "filter-button"}
        aria-haspopup="dialog"
        aria-expanded={open}
        title={title}
        onClick={() => setOpen((o) => !o)}
      >
        {icon}
        <span className="filter-button-label">{label}</span>
        <svg className="chevron" viewBox="0 0 12 12" aria-hidden="true">
          <path d="M3 4.5l3 3 3-3" fill="none" stroke="currentColor" strokeWidth="1.5" />
        </svg>
      </button>
      {open && (
        <Panel root={root} label={title} onClose={close}>
          {children}
        </Panel>
      )}
    </div>
  );
}

function Panel({
  root,
  label,
  onClose,
  children,
}: {
  root: RefObject<HTMLDivElement | null>;
  label: string;
  onClose: () => void;
  children: ReactNode;
}) {
  useDismiss(root, onClose);
  return (
    <div className="filter-panel" role="dialog" aria-label={label}>
      {children}
    </div>
  );
}

function CalendarGlyph() {
  return (
    <svg className="filter-glyph" viewBox="0 0 16 16" aria-hidden="true">
      <rect x="2" y="3" width="12" height="11" rx="2.5" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <path d="M2 6.5h12M5.25 1.75v2.5M10.75 1.75v2.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  );
}

function TagGlyph() {
  return (
    <svg className="filter-glyph" viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M2 3.25C2 2.56 2.56 2 3.25 2h4.13c.33 0 .65.13.88.37l5.37 5.37a1.25 1.25 0 0 1 0 1.77l-4.12 4.12a1.25 1.25 0 0 1-1.77 0L2.37 8.26A1.25 1.25 0 0 1 2 7.38z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
      />
      <circle cx="5.25" cy="5.25" r="1.1" fill="currentColor" />
    </svg>
  );
}
