import type { Filters, Kind } from "../model/search";

/**
 * When a result last changed, as the date filter offers it: a preset relative to
 * today, or a custom day, month, year or range typed by the user. Custom values are
 * the strings the date inputs hold (`2026-03-05`, `2026-03`, `2026`).
 */
export type DateChoice =
  | { kind: "any" }
  | { kind: "today" }
  | { kind: "yesterday" }
  | { kind: "last7" }
  | { kind: "last30" }
  | { kind: "thisMonth" }
  | { kind: "thisYear" }
  | { kind: "day"; date: string }
  | { kind: "month"; month: string }
  | { kind: "year"; year: string }
  | { kind: "range"; from: string; to: string };

/** Everything the filter bar holds. */
export type FilterState = {
  date: DateChoice;
  kind: Kind;
  /** Folders must carry every one of these. */
  tags: string[];
};

export const initialFilters: FilterState = { date: { kind: "any" }, kind: "any", tags: [] };

/** The presets, in the order the date menu lists them. */
export const DATE_PRESETS: { choice: DateChoice; label: string }[] = [
  { choice: { kind: "any" }, label: "Any date" },
  { choice: { kind: "today" }, label: "Today" },
  { choice: { kind: "yesterday" }, label: "Yesterday" },
  { choice: { kind: "last7" }, label: "Last 7 days" },
  { choice: { kind: "last30" }, label: "Last 30 days" },
  { choice: { kind: "thisMonth" }, label: "This month" },
  { choice: { kind: "thisYear" }, label: "This year" },
];

/** A span of local time in ms since the epoch: `from` included, `to` excluded. */
export type Span = { from: number | null; to: number | null };

const NONE: Span = { from: null, to: null };

/** Local midnight of the given day; JavaScript rolls day 0 or 32 over correctly. */
const midnight = (y: number, m: number, d: number) => new Date(y, m, d).getTime();

/** `2026-03-05` as a local date, or null when it is not a real day. */
function parseDay(text: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text);
  if (!match) {
    return null;
  }
  const [y, m, d] = [Number(match[1]), Number(match[2]) - 1, Number(match[3])];
  const date = new Date(y, m, d);
  return date.getFullYear() === y && date.getMonth() === m && date.getDate() === d ? date : null;
}

/** `2026-03` as [year, month index], or null. */
function parseMonth(text: string): [number, number] | null {
  const match = /^(\d{4})-(\d{2})$/.exec(text);
  const month = Number(match?.[2]) - 1;
  return match && month >= 0 && month <= 11 ? [Number(match[1]), month] : null;
}

function parseYear(text: string): number | null {
  return /^\d{4}$/.test(text) ? Number(text) : null;
}

/**
 * The span of local time a date choice covers, measured from `now`. "Last 7 days"
 * includes today. An incomplete or invalid custom value narrows nothing.
 */
export function dateRange(choice: DateChoice, now: Date = new Date()): Span {
  const [y, m, d] = [now.getFullYear(), now.getMonth(), now.getDate()];
  switch (choice.kind) {
    case "any":
      return NONE;
    case "today":
      return { from: midnight(y, m, d), to: midnight(y, m, d + 1) };
    case "yesterday":
      return { from: midnight(y, m, d - 1), to: midnight(y, m, d) };
    case "last7":
      return { from: midnight(y, m, d - 6), to: midnight(y, m, d + 1) };
    case "last30":
      return { from: midnight(y, m, d - 29), to: midnight(y, m, d + 1) };
    case "thisMonth":
      return { from: midnight(y, m, 1), to: midnight(y, m + 1, 1) };
    case "thisYear":
      return { from: midnight(y, 0, 1), to: midnight(y + 1, 0, 1) };
    case "day": {
      const day = parseDay(choice.date);
      return day ? { from: day.getTime(), to: nextMidnight(day) } : NONE;
    }
    case "month": {
      const month = parseMonth(choice.month);
      if (!month) {
        return NONE;
      }
      const [year, index] = month;
      return { from: midnight(year, index, 1), to: midnight(year, index + 1, 1) };
    }
    case "year": {
      const year = parseYear(choice.year);
      return year === null ? NONE : { from: midnight(year, 0, 1), to: midnight(year + 1, 0, 1) };
    }
    case "range": {
      let [from, to] = [parseDay(choice.from), parseDay(choice.to)];
      if (from && to && from > to) {
        [from, to] = [to, from];
      }
      return {
        from: from ? from.getTime() : null,
        to: to ? nextMidnight(to) : null,
      };
    }
  }
}

/** The midnight that ends `day`. */
function nextMidnight(day: Date): number {
  return midnight(day.getFullYear(), day.getMonth(), day.getDate() + 1);
}

const DAY = new Intl.DateTimeFormat("en-US", { month: "short", day: "numeric", year: "numeric" });
const MONTH = new Intl.DateTimeFormat("en-US", { month: "long", year: "numeric" });

/** The date filter's button label. */
export function describeDate(choice: DateChoice): string {
  const preset = DATE_PRESETS.find((p) => p.choice.kind === choice.kind);
  if (preset) {
    return preset.label;
  }
  if (dateRange(choice).from === null && dateRange(choice).to === null) {
    return "Any date";
  }
  switch (choice.kind) {
    case "day":
      return DAY.format(parseDay(choice.date) ?? 0);
    case "month": {
      const [y, m] = parseMonth(choice.month) ?? [0, 0];
      return MONTH.format(new Date(y, m, 1));
    }
    case "year":
      return choice.year;
    case "range": {
      const [from, to] = [parseDay(choice.from), parseDay(choice.to)];
      if (from && to) {
        const [a, b] = from > to ? [to, from] : [from, to];
        return `${DAY.format(a)} – ${DAY.format(b)}`;
      }
      return from ? `Since ${DAY.format(from)}` : `Until ${DAY.format(to ?? 0)}`;
    }
    default:
      return "Any date";
  }
}

/** What Rust narrows by, with the date measured from `now`. */
export function toFilters(state: FilterState, now: Date = new Date()): Filters {
  const span = dateRange(state.date, now);
  return { kind: state.kind, tags: state.tags, modifiedFrom: span.from, modifiedTo: span.to };
}

/** True when the date choice narrows anything; a custom date still being typed does not. */
export function narrowsDate(choice: DateChoice): boolean {
  const span = dateRange(choice);
  return span.from !== null || span.to !== null;
}

/** True once any filter narrows the results. */
export function isFiltered(state: FilterState): boolean {
  return state.kind !== "any" || state.tags.length > 0 || narrowsDate(state.date);
}

/** The custom date kinds, in the order the date menu's segmented control shows them. */
export const CUSTOM_DATES: { kind: "day" | "month" | "year" | "range"; label: string }[] = [
  { kind: "day", label: "Day" },
  { kind: "month", label: "Month" },
  { kind: "year", label: "Year" },
  { kind: "range", label: "Range" },
];

/** An empty custom choice of `kind`, ready for its input to fill. */
export function emptyCustom(kind: "day" | "month" | "year" | "range"): DateChoice {
  switch (kind) {
    case "day":
      return { kind, date: "" };
    case "month":
      return { kind, month: "" };
    case "year":
      return { kind, year: "" };
    case "range":
      return { kind, from: "", to: "" };
  }
}

export function toggleTag(tags: string[], tag: string): string[] {
  return tags.includes(tag) ? tags.filter((t) => t !== tag) : [...tags, tag];
}
