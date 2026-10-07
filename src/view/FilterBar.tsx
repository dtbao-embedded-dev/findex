import { useCallback, useEffect, useRef, useState, type ReactNode, type RefObject } from "react";

import type { Kind } from "../model/search";
import type { TagInfo } from "../model/tags";
import {
  clearFilters,
  CUSTOM_DATES,
  DATE_PRESETS,
  describeDate,
  describeExts,
  emptyCustom,
  EXT_GROUPS,
  formatExts,
  groupOn,
  isFiltered,
  narrowsDate,
  parseExts,
  toggleGroup,
  toggleTag,
  type DateChoice,
  type FilterState,
  type SearchMode,
} from "../presenter/filterPresenter";
import { tagColors, tagLabel, tagName } from "../presenter/tagPresenter";
import { TagMark } from "./TagMark";
import { useDismiss } from "./useDismiss";

type Props = {
  filters: FilterState;
  /** Every tag that can be picked, offered by the tag menu. */
  tags: TagInfo[];
  onChange: (filters: FilterState) => void;
};

const MODES: { mode: SearchMode; label: string; title: string }[] = [
  { mode: "name", label: "Name", title: "Find the words in file and folder names" },
  {
    mode: "contents",
    label: "Contents",
    title: "Find the words in names and inside text files; results arrive as files are read",
  },
];

const KINDS: { kind: Kind; label: string }[] = [
  { kind: "any", label: "All" },
  { kind: "file", label: "Files" },
  { kind: "folder", label: "Folders" },
];

/**
 * The bar under the search field: first where the words are looked for (names, or
 * names and contents, as Finder offers), then the filters: when a result last
 * changed, whether it is a file or a folder, its file type, and the tags a folder
 * carries. Each change searches again at once; nothing waits for an Apply button.
 */
export function FilterBar({ filters, tags, onChange }: Props) {
  const set = (change: Partial<FilterState>) => onChange({ ...filters, ...change });
  const colors = tagColors(tags);

  return (
    <div className="filter-bar" role="toolbar" aria-label="Filters">
      <div className="segmented" role="radiogroup" aria-label="Search in">
        {MODES.map((m) => (
          <button
            key={m.mode}
            type="button"
            role="radio"
            aria-checked={filters.mode === m.mode}
            title={m.title}
            onClick={() => set({ mode: m.mode })}
          >
            {m.label}
          </button>
        ))}
      </div>

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
        label={describeExts(filters.exts)}
        active={filters.exts.length > 0}
        icon={<TypeGlyph />}
        title="File types to keep"
      >
        <TypeMenu exts={filters.exts} onChoose={(exts) => set({ exts })} />
      </Popover>

      <Popover
        label={
          filters.tags.length === 0 ? "Tags" : filters.tags.map((t) => tagLabel(t, colors)).join(", ")
        }
        active={filters.tags.length > 0}
        icon={<TagGlyph />}
        title="Folders carrying every tag picked"
      >
        {tags.length === 0 ? (
          <p className="filter-note">The tags could not be read.</p>
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
                <TagMark tag={t.tag} colors={colors} />
                <span className="filter-option-label">{tagName(t.tag, colors)}</span>
                {t.folders > 0 && <span className="sidebar-count">{t.folders}</span>}
              </button>
            ))}
          </div>
        )}
      </Popover>

      {isFiltered(filters) && (
        <button type="button" className="filter-clear" onClick={() => onChange(clearFilters(filters))}>
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

/**
 * The type menu: groups of common types to pick with one press, then a field for
 * any extensions. The field always shows every type picked, so a group's types can
 * be trimmed there; it applies on Enter or when it loses focus.
 */
function TypeMenu({ exts, onChoose }: { exts: string[]; onChoose: (exts: string[]) => void }) {
  const [draft, setDraft] = useState(formatExts(exts));
  useEffect(() => setDraft(formatExts(exts)), [exts.join(",")]);
  const commit = () => {
    const next = parseExts(draft);
    if (next.join(",") !== exts.join(",")) {
      onChoose(next);
    }
  };
  return (
    <>
      <div className="filter-options" role="group" aria-label="File type groups">
        {EXT_GROUPS.map((g) => (
          <button
            key={g.label}
            type="button"
            role="menuitemcheckbox"
            aria-checked={groupOn(exts, g)}
            className="filter-option"
            title={formatExts(g.exts)}
            onClick={() => onChoose(toggleGroup(exts, g))}
          >
            <span className="filter-check" aria-hidden="true" />
            <span className="filter-option-label">{g.label}</span>
          </button>
        ))}
      </div>
      <div className="filter-custom">
        <label className="filter-heading" htmlFor="filter-exts">
          Extensions
        </label>
        <input
          id="filter-exts"
          type="text"
          spellCheck={false}
          autoComplete="off"
          placeholder=".c, .h"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              commit();
            }
          }}
        />
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

function TypeGlyph() {
  return (
    <svg className="filter-glyph" viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M4.25 1.75h4.5l3.5 3.5v7.5c0 .83-.67 1.5-1.5 1.5h-6.5c-.83 0-1.5-.67-1.5-1.5v-9.5c0-.83.67-1.5 1.5-1.5z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinejoin="round"
      />
      <path d="M8.75 1.75v3.5h3.5" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round" />
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
