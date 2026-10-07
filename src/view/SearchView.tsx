import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import type { Details } from "../model/details";
import type { Hit } from "../model/search";
import type { TagInfo } from "../model/tags";
import type { Rows } from "../presenter/listPresenter";
import {
  canSearch,
  describeResults,
  initialSearch,
  keepSelection,
  latestDetails,
  latestOnly,
  moveSelection,
  resultsView,
  revealHit,
} from "../presenter/searchPresenter";
import { searchPlaceholder } from "../presenter/drivePresenter";
import { menuFor, runAction, type MenuAction } from "../presenter/menuPresenter";
import {
  initialFilters,
  isFiltered,
  toFilters,
  toggleTag,
  type FilterState,
} from "../presenter/filterPresenter";
import { keepKnown, tagColors } from "../presenter/tagPresenter";
import { ContextMenu } from "./ContextMenu";
import { FilterBar } from "./FilterBar";
import { ResultList } from "./ResultList";
import { Sidebar } from "./Sidebar";
import { TagEditor } from "./TagEditor";
import { Toast } from "./Toast";

/**
 * The open right-click menu: where it was asked for, and for which result. The result
 * itself, not its row: live updates can reorder the rows while the menu is open.
 */
type OpenMenu = { at: { x: number; y: number }; hit: Hit };

/** The folder whose tags are being edited, where its menu was, and its tags now. */
type Editing = { at: { x: number; y: number }; folder: Hit; tags: string[] };

/** A short message over the status bar; `id` restarts its timer when it repeats. */
type Notice = { text: string; tone: "info" | "error"; id: number };

type Props = {
  /** How many names are indexed, for the empty state. */
  total: number;
  /** The drive the search covers. */
  active: string | null;
  onActiveChange: (active: string | null) => void;
  /** How many results a search shows, from the settings. */
  limit: number;
  /** Every tag that can be picked. */
  tags: TagInfo[];
  /** Asks for the tags again after a folder's tags changed. */
  onTagsChanged: () => void;
};

/**
 * The search screen: one field, results as you type, keyboard first. ↑/↓ move the
 * selection, Enter (or a double click) shows the result in Explorer, Esc clears.
 * Right-click, the menu key or Shift+F10 opens the actions for a result.
 */
export function SearchView({ total, active, onActiveChange, limit, tags, onTagsChanged }: Props) {
  const [state, setState] = useState(initialSearch);
  const [filters, setFilters] = useState<FilterState>(initialFilters);
  const search = useMemo(() => latestOnly(), []);
  const loadDetails = useMemo(() => latestDetails(), []);
  const [details, setDetails] = useState<Record<string, Details>>({});
  // The rows in view, whose details are read; the rest are read as they scroll in.
  const [shown, setShown] = useState<Rows>({ first: 0, last: 0 });
  const detailsFor = useRef(state.hits);
  const field = useRef<HTMLInputElement>(null);
  const rows = useRef<(HTMLLIElement | null)[]>([]);
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [editing, setEditing] = useState<Editing | null>(null);
  // Set when the keyboard opened the menu, so the field's own menu stays shut.
  const menuByKey = useRef(false);

  const show = (text: string, tone: Notice["tone"]) =>
    setNotice((n) => ({ text, tone, id: (n?.id ?? 0) + 1 }));

  useEffect(() => {
    if (notice === null) {
      return;
    }
    const timer = setTimeout(() => setNotice(null), notice.tone === "error" ? 4000 : 1800);
    return () => clearTimeout(timer);
  }, [notice]);

  const closeMenu = useCallback(() => {
    setMenu(null);
    field.current?.focus();
  }, []);

  const closeEditor = useCallback(() => {
    setEditing(null);
    field.current?.focus();
  }, []);

  const tagsChanged = (path: string, folderTags: string[]) => {
    setDetails((d) => {
      const known = d[path];
      return known ? { ...d, [path]: { ...known, tags: folderTags } } : d;
    });
    onTagsChanged();
  };

  // A tag deleted in the settings stops narrowing the search.
  useEffect(() => {
    const kept = keepKnown(filters.tags, tags);
    if (kept.length !== filters.tags.length) {
      changeFilters({ ...filters, tags: kept });
    }
  }, [tags]);

  // Coming back from the tray or another window, typing goes straight to the field.
  useEffect(() => {
    const focus = () => field.current?.focus();
    window.addEventListener("focus", focus);
    return () => window.removeEventListener("focus", focus);
  }, []);

  // Details follow the rows in view; until they arrive a row keeps what it showed
  // before. New results replace the details read so far, a scroll adds to them.
  useEffect(() => {
    const fresh = detailsFor.current !== state.hits;
    detailsFor.current = state.hits;
    void loadDetails(state.hits.slice(shown.first, shown.last)).then((found) => {
      if (found) {
        setDetails((d) => (fresh ? found : { ...d, ...found }));
      }
    });
    // The tags change too: a deleted tag must leave the rows that showed it.
  }, [state.hits, shown, loadDetails, tags]);

  // Dates such as "Today" are measured when each search is asked, not when picked.
  const run = (query: string, by: FilterState = filters) => {
    setState((s) => ({ ...s, query }));
    void search(query, toFilters(by)).then((update) => {
      if (update) {
        setState((s) => ({ ...s, ...update }));
      }
    });
  };

  const changeFilters = (next: FilterState) => {
    setFilters(next);
    run(state.query, next);
  };

  // A drive finishing, its snapshot loading, or files changing on disk change what
  // can be found: answer the query on screen again, keeping the selected file.
  useEffect(() => {
    if (!canSearch(state.query, filters)) {
      return;
    }
    void search(state.query, toFilters(filters)).then((update) => {
      if (update) {
        setState((s) => ({
          ...s,
          ...update,
          selected: keepSelection(s.hits[s.selected], update.hits),
        }));
      }
    });
    // Only a change in the searchable count or the results limit refreshes here;
    // typing runs on its own.
  }, [total, limit]);

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

  const openMenuAtSelection = () => {
    const row = rows.current[state.selected];
    const hit = state.hits[state.selected];
    if (row && hit) {
      const box = row.getBoundingClientRect();
      setMenu({ at: { x: box.left + 40, y: box.bottom }, hit });
    }
  };

  const pick = (action: MenuAction) => {
    const hit = menu?.hit;
    closeMenu();
    if (hit && menu && action === "tags") {
      setEditing({ at: menu.at, folder: hit, tags: details[hit.path]?.tags ?? [] });
    } else if (hit) {
      void runAction(action, hit).then((result) => {
        if (result.error !== null) {
          show(result.error, "error");
        } else if (result.notice !== null) {
          show(result.notice, "info");
        }
      });
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) {
      event.preventDefault();
      menuByKey.current = true;
      openMenuAtSelection();
      return;
    }
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

  const view = resultsView(state, filters);
  const colors = useMemo(() => tagColors(tags), [tags]);

  const searchTag = (tag: string) => {
    changeFilters({ ...filters, tags: toggleTag(filters.tags, tag) });
    field.current?.focus();
  };

  return (
    <div className="workspace">
      <Sidebar
        active={active}
        onActiveChange={(letter, error) => {
          onActiveChange(letter);
          if (error !== null) {
            show(error, "error");
          }
          field.current?.focus();
        }}
        tags={tags}
        selectedTags={filters.tags}
        onTag={searchTag}
      />
      <main className="search">
        <div className="search-header">
          <label className="search-field">
            <SearchGlyph />
            <input
              ref={field}
              autoFocus
              type="text"
              spellCheck={false}
              autoComplete="off"
              placeholder={searchPlaceholder(active)}
              aria-label="Search files and folders"
              aria-controls="results"
              value={state.query}
              onChange={(event) => run(event.target.value)}
              onKeyDown={onKeyDown}
              onContextMenu={(event) => {
                if (menuByKey.current) {
                  event.preventDefault();
                  menuByKey.current = false;
                }
              }}
            />
          </label>
          <FilterBar filters={filters} tags={tags} onChange={changeFilters} />
        </div>

        {view === "list" ? (
          <ResultList
            hits={state.hits}
            selected={state.selected}
            query={state.query}
            details={details}
            meta={
              state.tookMs === null
                ? null
                : describeResults(state.hits.length, state.tookMs, {
                    partial: state.partial,
                    limit,
                  })
            }
            rows={rows}
            onSelect={(index) => setState((s) => ({ ...s, selected: index }))}
            onReveal={reveal}
            onMenu={(at, index) => {
              const hit = state.hits[index];
              setState((s) => ({ ...s, selected: index }));
              if (hit) {
                setMenu({ at, hit });
              }
            }}
            onTag={searchTag}
            tagColors={colors}
            onDetailsWindow={setShown}
          />
        ) : (
          <div className="results-scroll">
            {view === "error" && (
              <div className="empty">
                <p className="error">{state.error}</p>
              </div>
            )}
            {view === "empty" && (
              <div className="empty">
                <EmptyGlyph />
                <p className="empty-title">
                  Search {total.toLocaleString("en-US")} items{active !== null && ` on ${active}:`}
                </p>
                <p className="empty-hint">
                  {isFiltered(filters) ? (
                    <>Type part of a name to search with these filters.</>
                  ) : (
                    <>
                      Type part of a name, then narrow with the filters above or{" "}
                      <kbd>ext:pdf</kbd>.
                    </>
                  )}
                </p>
              </div>
            )}
            {view === "none" && (
              <div className="empty">
                <p className="empty-title">No matches for &ldquo;{state.query.trim()}&rdquo;</p>
              </div>
            )}
          </div>
        )}

        {menu !== null && (
          <ContextMenu
            at={menu.at}
            items={menuFor(menu.hit)}
            onPick={pick}
            onClose={closeMenu}
          />
        )}
        {editing !== null && (
          <TagEditor
            at={editing.at}
            folder={editing.folder}
            tags={editing.tags}
            catalog={tags}
            onChange={(tags) => {
              setEditing((e) => e && { ...e, tags });
              tagsChanged(editing.folder.path, tags);
            }}
            onClose={closeEditor}
          />
        )}
        {notice !== null && <Toast key={notice.id} text={notice.text} tone={notice.tone} />}
      </main>
    </div>
  );
}

/** A large, quiet magnifier over the empty screen. */
function EmptyGlyph() {
  return (
    <svg className="empty-glyph" viewBox="0 0 48 48" aria-hidden="true">
      <circle cx="21" cy="21" r="13" fill="currentColor" opacity="0.12" />
      <circle cx="21" cy="21" r="13" fill="none" stroke="currentColor" strokeWidth="2.5" />
      <path d="M31 31l9 9" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
    </svg>
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
