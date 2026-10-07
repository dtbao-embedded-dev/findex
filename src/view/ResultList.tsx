import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";

import type { Details } from "../model/details";
import type { Hit } from "../model/search";
import { detailsWindow, scrollToShow, visibleRange, type Rows } from "../presenter/listPresenter";
import { describeHit, folderOf, highlight } from "../presenter/searchPresenter";
import { tagLabel } from "../presenter/tagPresenter";

type Props = {
  hits: Hit[];
  selected: number;
  query: string;
  details: Record<string, Details>;
  /** The line above the rows, such as "Top 200 results · 12 ms"; null while searching. */
  meta: string | null;
  /** The rendered rows by index, for placing the menu at the selected one. */
  rows: RefObject<(HTMLLIElement | null)[]>;
  onSelect: (index: number) => void;
  onMenu: (at: { x: number; y: number }, index: number) => void;
  onTag: (tag: string) => void;
  /** The colour of each preset tag, by name. */
  tagColors: Record<string, string>;
  /** Changes when the selected row must be scrolled back into view. */
  reveal: number;
  /** The rows whose details are worth reading, reported as the view scrolls. */
  onDetailsWindow: (rows: Rows) => void;
};

/** A first guess at a row's height (`--row`), replaced by the measured one. */
const ROW_GUESS = 44;

/**
 * The results, as a list that renders only the rows in view: up to 10,000 results
 * stay as quick to show and scroll as 200. Every row has the same height, so the
 * list keeps its full scroll length while most of it is empty space.
 */
export function ResultList({
  hits,
  selected,
  query,
  details,
  meta,
  rows,
  onSelect,
  onMenu,
  onTag,
  tagColors,
  reveal,
  onDetailsWindow,
}: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const [view, setView] = useState({ scrollTop: 0, height: 0 });
  const [rowHeight, setRowHeight] = useState(ROW_GUESS);
  const [listTop, setListTop] = useState(0);

  // The visible height follows the window; the scroll position follows the user.
  useLayoutEffect(() => {
    const el = scroller.current;
    if (!el) {
      return;
    }
    const measure = () => setView({ scrollTop: el.scrollTop, height: el.clientHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Where the list starts (below the count line, which can wrap) and how tall a row
  // really is; measured again when the line or the width changes.
  useLayoutEffect(() => {
    const measure = () => {
      setListTop(list.current?.offsetTop ?? 0);
      const row = list.current?.querySelector("li");
      if (row) {
        setRowHeight(row.getBoundingClientRect().height || ROW_GUESS);
      }
    };
    measure();
    const el = scroller.current;
    if (!el) {
      return;
    }
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [meta, hits.length > 0]);

  // The selected row stays in view as the keyboard moves it or results refresh.
  useLayoutEffect(() => {
    const el = scroller.current;
    if (!el) {
      return;
    }
    const next = scrollToShow(selected, rowHeight, listTop, el.scrollTop, el.clientHeight);
    if (next !== null) {
      el.scrollTop = next;
      setView({ scrollTop: el.scrollTop, height: el.clientHeight });
    }
  }, [selected, hits, rowHeight, listTop, reveal]);

  const range = visibleRange(view.scrollTop, view.height, rowHeight, listTop, hits.length);
  const wanted = detailsWindow(range.first, range.last, hits.length);

  useEffect(() => {
    onDetailsWindow(wanted);
  }, [wanted.first, wanted.last, onDetailsWindow]);

  return (
    <div
      ref={scroller}
      className="results-scroll"
      onScroll={(event) =>
        setView({ scrollTop: event.currentTarget.scrollTop, height: event.currentTarget.clientHeight })
      }
    >
      {meta !== null && (
        <p
          className="results-meta"
          title="Results found, the time the search took, and the memory the drive's name index holds"
        >
          {meta}
        </p>
      )}
      <ul
        ref={list}
        id="results"
        className="results"
        role="listbox"
        aria-label="Results"
        style={{ height: hits.length * rowHeight }}
      >
        {hits.slice(range.first, range.last).map((hit, offset) => {
          const index = range.first + offset;
          return (
            <li
              key={hit.path}
              ref={(el) => {
                rows.current[index] = el;
              }}
              role="option"
              aria-selected={index === selected}
              aria-setsize={hits.length}
              aria-posinset={index + 1}
              className={index === selected ? "result is-selected" : "result"}
              style={{ top: index * rowHeight }}
              onMouseDown={() => onSelect(index)}
              onContextMenu={(event) => {
                event.preventDefault();
                onMenu({ x: event.clientX, y: event.clientY }, index);
              }}
            >
              <KindGlyph folder={hit.isDir} />
              <span className="result-text">
                <span className="result-line">
                  <span className="result-name">
                    {highlight(hit.name, query).map((part, i) =>
                      part.match ? <mark key={i}>{part.text}</mark> : part.text,
                    )}
                  </span>
                  {details[hit.path]?.tags.map((tag) => (
                    <button
                      key={tag}
                      type="button"
                      className="tag-chip is-small"
                      tabIndex={-1}
                      title={`Filter by the tag ${tagLabel(tag, tagColors)}`}
                      onClick={() => onTag(tag)}
                    >
                      {tag in tagColors && (
                        <span className="tag-dot" data-color={tagColors[tag]} aria-hidden="true" />
                      )}
                      {tagLabel(tag, tagColors)}
                    </button>
                  ))}
                </span>
                {hit.line === undefined ? (
                  <span className="result-path">{folderOf(hit.path)}</span>
                ) : (
                  <span className="result-path is-snippet" title={folderOf(hit.path)}>
                    <span className="result-line-no" aria-label={`Line ${hit.line}`}>
                      {hit.line}
                    </span>
                    {highlight(hit.snippet ?? "", query).map((part, i) =>
                      part.match ? <mark key={i}>{part.text}</mark> : part.text,
                    )}
                  </span>
                )}
              </span>
              <HitMeta meta={describeHit(details[hit.path])} />
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** Type and size over the last-changed date, at the right of a result. */
function HitMeta({ meta }: { meta: { kind: string; modified: string } | null }) {
  if (meta === null) {
    return null;
  }
  return (
    <span className="result-meta">
      <span>{meta.kind}</span>
      <span>{meta.modified}</span>
    </span>
  );
}

function KindGlyph({ folder }: { folder: boolean }) {
  return folder ? (
    <svg className="kind-glyph is-folder" viewBox="0 0 24 24" aria-label="Folder">
      <path
        d="M3 7.5A2.5 2.5 0 0 1 5.5 5h3.6l2 2h7.4A2.5 2.5 0 0 1 21 9.5v7A2.5 2.5 0 0 1 18.5 19h-13A2.5 2.5 0 0 1 3 16.5z"
        fill="currentColor"
      />
    </svg>
  ) : (
    <svg className="kind-glyph" viewBox="0 0 24 24" aria-label="File">
      <path
        d="M7 3h6.5L19 8.5V19a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"
        fill="currentColor"
        opacity="0.22"
      />
      <path d="M13.5 3v5.5H19" fill="none" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}
