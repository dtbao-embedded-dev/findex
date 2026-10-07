import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import {
  describeResults,
  folderOf,
  highlight,
  initialSearch,
  keepSelection,
  latestOnly,
  moveSelection,
  revealHit,
} from "../presenter/searchPresenter";

type Props = {
  /** How many names are indexed, for the empty state. */
  total: number;
  onOpenDrives: () => void;
};

/**
 * The search screen: one field, results as you type, keyboard first. ↑/↓ move the
 * selection, Enter (or a double click) shows the result in Explorer, Esc clears.
 */
export function SearchView({ total, onOpenDrives }: Props) {
  const [state, setState] = useState(initialSearch);
  const search = useMemo(() => latestOnly(), []);
  const field = useRef<HTMLInputElement>(null);
  const rows = useRef<(HTMLLIElement | null)[]>([]);

  // Coming back from the tray or another window, typing goes straight to the field.
  useEffect(() => {
    const focus = () => field.current?.focus();
    window.addEventListener("focus", focus);
    return () => window.removeEventListener("focus", focus);
  }, []);

  useEffect(() => {
    rows.current[state.selected]?.scrollIntoView({ block: "nearest" });
  }, [state.selected, state.hits]);

  const run = (query: string) => {
    setState((s) => ({ ...s, query }));
    void search(query).then((update) => {
      if (update) {
        setState((s) => ({ ...s, ...update }));
      }
    });
  };

  // A drive finishing, its snapshot loading, or files changing on disk change what
  // can be found: answer the query on screen again, keeping the selected file.
  useEffect(() => {
    if (state.query.trim() === "") {
      return;
    }
    void search(state.query).then((update) => {
      if (update) {
        setState((s) => ({
          ...s,
          ...update,
          selected: keepSelection(s.hits[s.selected], update.hits),
        }));
      }
    });
    // Only a change in the searchable count refreshes here; typing runs on its own.
  }, [total]);

  const reveal = (index: number) => {
    const hit = state.hits[index];
    if (hit) {
      void revealHit(hit).then((error) => {
        if (error) {
          setState((s) => ({ ...s, error }));
        }
      });
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    const step = { ArrowDown: 1, ArrowUp: -1 }[event.key as "ArrowDown" | "ArrowUp"];
    if (step !== undefined) {
      event.preventDefault();
      setState((s) => ({ ...s, selected: moveSelection(s.selected, step, s.hits.length) }));
    } else if (event.key === "Enter") {
      reveal(state.selected);
    } else if (event.key === "Escape") {
      run("");
    }
  };

  const hasQuery = state.query.trim() !== "";

  return (
    <div className="search">
      <div className="toolbar">
        <label className="search-field">
          <SearchGlyph />
          <input
            ref={field}
            autoFocus
            type="text"
            spellCheck={false}
            autoComplete="off"
            placeholder="Search files and folders"
            aria-label="Search files and folders"
            aria-controls="results"
            value={state.query}
            onChange={(event) => run(event.target.value)}
            onKeyDown={onKeyDown}
          />
        </label>
        <button type="button" className="button-secondary" onClick={onOpenDrives}>
          Drives
        </button>
      </div>

      <div className="results-scroll">
        {state.error !== null ? (
          <div className="empty">
            <p className="error">{state.error}</p>
          </div>
        ) : !hasQuery ? (
          <div className="empty">
            <p className="empty-title">Search {total.toLocaleString("en-US")} items</p>
            <p className="empty-hint">
              Type part of a name. Narrow with <kbd>ext:pdf</kbd> or <kbd>type:folder</kbd>.
            </p>
          </div>
        ) : state.hits.length === 0 && state.tookMs !== null ? (
          <div className="empty">
            <p className="empty-title">No matches for &ldquo;{state.query.trim()}&rdquo;</p>
          </div>
        ) : (
          <>
            {state.tookMs !== null && (
              <p className="results-meta">{describeResults(state.hits.length, state.tookMs)}</p>
            )}
            <ul id="results" className="results" role="listbox" aria-label="Results">
              {state.hits.map((hit, index) => (
                <li
                  key={hit.path}
                  ref={(el) => {
                    rows.current[index] = el;
                  }}
                  role="option"
                  aria-selected={index === state.selected}
                  className={index === state.selected ? "result is-selected" : "result"}
                  onMouseDown={() => setState((s) => ({ ...s, selected: index }))}
                  onDoubleClick={() => reveal(index)}
                >
                  <KindGlyph folder={hit.isDir} />
                  <span className="result-text">
                    <span className="result-name">
                      {highlight(hit.name, state.query).map((part, i) =>
                        part.match ? <mark key={i}>{part.text}</mark> : part.text,
                      )}
                    </span>
                    <span className="result-path">{folderOf(hit.path)}</span>
                  </span>
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
    </div>
  );
}

function SearchGlyph() {
  return (
    <svg className="search-glyph" viewBox="0 0 20 20" aria-hidden="true">
      <circle cx="8.5" cy="8.5" r="5.75" fill="none" stroke="currentColor" strokeWidth="1.75" />
      <path d="M13 13l4.25 4.25" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
    </svg>
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
