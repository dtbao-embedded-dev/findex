import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import type { Details } from "../model/details";
import type { Hit } from "../model/search";
import type { TagCount } from "../model/tags";
import {
  describeHit,
  describeResults,
  folderOf,
  highlight,
  initialSearch,
  keepSelection,
  latestDetails,
  latestOnly,
  moveSelection,
  revealHit,
} from "../presenter/searchPresenter";
import { searchPlaceholder } from "../presenter/drivePresenter";
import { menuFor, runAction, type MenuAction } from "../presenter/menuPresenter";
import { loadTags, tagQuery } from "../presenter/tagPresenter";
import { ContextMenu } from "./ContextMenu";
import { DrivePicker } from "./DrivePicker";
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
};

/**
 * The search screen: one field, results as you type, keyboard first. ↑/↓ move the
 * selection, Enter (or a double click) shows the result in Explorer, Esc clears.
 * Right-click, the menu key or Shift+F10 opens the actions for a result.
 */
export function SearchView({ total, active, onActiveChange }: Props) {
  const [state, setState] = useState(initialSearch);
  const search = useMemo(() => latestOnly(), []);
  const loadDetails = useMemo(() => latestDetails(), []);
  const [details, setDetails] = useState<Record<string, Details>>({});
  const field = useRef<HTMLInputElement>(null);
  const rows = useRef<(HTMLLIElement | null)[]>([]);
  const [menu, setMenu] = useState<OpenMenu | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [editing, setEditing] = useState<Editing | null>(null);
  const [tagsInUse, setTagsInUse] = useState<TagCount[]>([]);
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

  // The tags in use, offered on the empty screen; read again after every edit.
  useEffect(() => {
    void loadTags().then(setTagsInUse);
  }, []);

  const tagsChanged = (path: string, tags: string[]) => {
    setDetails((d) => {
      const known = d[path];
      return known ? { ...d, [path]: { ...known, tags } } : d;
    });
    void loadTags().then(setTagsInUse);
  };

  // Coming back from the tray or another window, typing goes straight to the field.
  useEffect(() => {
    const focus = () => field.current?.focus();
    window.addEventListener("focus", focus);
    return () => window.removeEventListener("focus", focus);
  }, []);

  useEffect(() => {
    rows.current[state.selected]?.scrollIntoView({ block: "nearest" });
  }, [state.selected, state.hits]);

  // Details follow the results; until they arrive a row keeps what it showed before.
  useEffect(() => {
    void loadDetails(state.hits).then((found) => {
      if (found) {
        setDetails(found);
      }
    });
  }, [state.hits, loadDetails]);

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

  const hasQuery = state.query.trim() !== "";

  return (
    <div className="search">
      <div className="toolbar">
        <DrivePicker
          active={active}
          onChange={(letter, error) => {
            onActiveChange(letter);
            if (error !== null) {
              show(error, "error");
            }
            field.current?.focus();
          }}
        />
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
      </div>

      <div className="results-scroll">
        {state.error !== null ? (
          <div className="empty">
            <p className="error">{state.error}</p>
          </div>
        ) : !hasQuery ? (
          <div className="empty">
            <p className="empty-title">
              Search {total.toLocaleString("en-US")} items{active !== null && ` on ${active}:`}
            </p>
            <p className="empty-hint">
              Type part of a name. Narrow with <kbd>ext:pdf</kbd>, <kbd>type:folder</kbd> or{" "}
              <kbd>tag:work</kbd>.
            </p>
            {tagsInUse.length > 0 && (
              <div className="empty-tags" aria-label="Tags">
                {tagsInUse.map((t) => (
                  <button
                    key={t.tag}
                    type="button"
                    className="tag-chip"
                    title={`${t.folders} folder${t.folders === 1 ? "" : "s"}`}
                    onClick={() => {
                      run(tagQuery(t.tag));
                      field.current?.focus();
                    }}
                  >
                    #{t.tag}
                    <span className="tag-count">{t.folders}</span>
                  </button>
                ))}
              </div>
            )}
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
                  onContextMenu={(event) => {
                    event.preventDefault();
                    setState((s) => ({ ...s, selected: index }));
                    setMenu({ at: { x: event.clientX, y: event.clientY }, hit });
                  }}
                >
                  <KindGlyph folder={hit.isDir} />
                  <span className="result-text">
                    <span className="result-line">
                      <span className="result-name">
                        {highlight(hit.name, state.query).map((part, i) =>
                          part.match ? <mark key={i}>{part.text}</mark> : part.text,
                        )}
                      </span>
                      {details[hit.path]?.tags.map((tag) => (
                        <button
                          key={tag}
                          type="button"
                          className="tag-chip is-small"
                          tabIndex={-1}
                          title={`Show folders tagged ${tag}`}
                          onClick={() => {
                            run(tagQuery(tag));
                            field.current?.focus();
                          }}
                        >
                          #{tag}
                        </button>
                      ))}
                    </span>
                    <span className="result-path">{folderOf(hit.path)}</span>
                  </span>
                  <HitMeta meta={describeHit(details[hit.path])} />
                </li>
              ))}
            </ul>
          </>
        )}
      </div>

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
          onChange={(tags) => {
            setEditing((e) => e && { ...e, tags });
            tagsChanged(editing.folder.path, tags);
          }}
          onClose={closeEditor}
        />
      )}
      {notice !== null && <Toast key={notice.id} text={notice.text} tone={notice.tone} />}
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
