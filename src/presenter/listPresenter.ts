/**
 * The arithmetic of a long result list that renders only the rows in view. Every
 * row has the same height, so a row's place follows from its index alone. Offsets
 * are in pixels within the scroll area; the list starts `listTop` pixels down it,
 * below the line that counts the results.
 */

/** Rows to render, `first` included and `last` excluded. */
export type Rows = { first: number; last: number };

/** The rows in view, plus `overscan` on each side so a fast scroll shows no gap. */
export function visibleRange(
  scrollTop: number,
  viewport: number,
  rowHeight: number,
  listTop: number,
  count: number,
  overscan = 8,
): Rows {
  const top = scrollTop - listTop;
  const first = Math.floor(top / rowHeight) - overscan;
  const last = Math.ceil((top + viewport) / rowHeight) + overscan;
  return {
    first: Math.min(count, Math.max(0, first)),
    last: Math.min(count, Math.max(0, last)),
  };
}

/**
 * The scroll position that brings row `index` fully into view, or null when it is
 * already. The first row scrolls to the very top, so the count above it shows too.
 */
export function scrollToShow(
  index: number,
  rowHeight: number,
  listTop: number,
  scrollTop: number,
  viewport: number,
): number | null {
  const top = listTop + index * rowHeight;
  const bottom = top + rowHeight;
  if (top < scrollTop) {
    return index === 0 ? 0 : top;
  }
  if (bottom > scrollTop + viewport) {
    return bottom - viewport;
  }
  return null;
}

/**
 * The rows whose details to read: those in view, rounded out to whole chunks, so a
 * scroll asks the disk again only when it crosses a chunk.
 */
export function detailsWindow(first: number, last: number, count: number, chunk = 50): Rows {
  return {
    first: Math.floor(first / chunk) * chunk,
    last: Math.min(count, Math.ceil(last / chunk) * chunk),
  };
}

/**
 * Whether to load the next page: one follows, and the rows drawn reach within `ahead`
 * rows of the last one loaded, so it arrives before the scroll gets there.
 */
export function needsMore(lastDrawn: number, loaded: number, more: boolean, ahead = 100): boolean {
  return more && loaded > 0 && lastDrawn >= loaded - ahead;
}
