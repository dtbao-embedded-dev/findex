import { useState } from "react";

import type { TagInfo } from "../model/tags";
import {
  createTagNamed,
  deletePrompt,
  deleteTagNamed,
  tagColors,
  tagName,
} from "../presenter/tagPresenter";

type Props = {
  catalog: TagInfo[];
  /** The catalog after a tag was created or deleted. */
  onCatalog: (catalog: TagInfo[]) => void;
};

/**
 * The Tags section of the settings: the preset tags, always there, then the user's
 * own tags with a field to make one and a delete button each. Deleting asks first,
 * since it takes the tag off every folder and cannot be undone.
 */
export function TagSettings({ catalog, onCatalog }: Props) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<string | null>(null);
  const colors = tagColors(catalog);
  const presets = catalog.filter((t) => t.preset);
  const custom = catalog.filter((t) => !t.preset);

  const apply = (edit: { catalog: TagInfo[] | null; error: string | null }) => {
    setError(edit.error);
    if (edit.catalog !== null) {
      onCatalog(edit.catalog);
      setText("");
      setConfirming(null);
    }
  };

  return (
    <section className="sheet-section" aria-labelledby="settings-tags">
      <h3 id="settings-tags" className="sheet-heading">
        Tags
      </h3>
      <div className="setting-group">
        <div className="setting-row">
          <div className="setting-text">
            <span className="setting-label">Preset tags</span>
            <p className="setting-hint">Always there, and cannot be deleted.</p>
          </div>
          <ul className="tag-colors is-static" aria-label="Preset tags">
            {presets.map((t) => (
              <li key={t.tag} className="tag-color">
                <span className="tag-dot" data-color={t.color ?? undefined} aria-hidden="true" />
                {tagName(t.tag, colors)}
              </li>
            ))}
          </ul>
        </div>
        <div className="setting-row is-stacked">
          <div className="setting-text">
            <label htmlFor="new-tag" className="setting-label">
              Your tags
            </label>
            <p className="setting-hint">One word of letters, digits, - or _. Up to 32 characters.</p>
          </div>
          <div className="setting-add">
            <input
              id="new-tag"
              type="text"
              spellCheck={false}
              autoComplete="off"
              placeholder="New tag"
              value={text}
              aria-invalid={error !== null}
              onChange={(event) => setText(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  void createTagNamed(text).then(apply);
                }
              }}
            />
            <button
              type="button"
              disabled={text.trim() === ""}
              onClick={() => void createTagNamed(text).then(apply)}
            >
              Add
            </button>
          </div>
          {error !== null && (
            <p className="error setting-error" role="alert">
              {error}
            </p>
          )}
          {custom.length === 0 ? (
            <p className="setting-hint">No tags of your own yet.</p>
          ) : (
            <ul className="setting-list" aria-label="Your tags">
              {custom.map((t) => (
                <li key={t.tag} className="setting-item">
                  {confirming === t.tag ? (
                    <>
                      <span className="setting-item-label">{deletePrompt(t)}</span>
                      <button type="button" className="is-quiet" onClick={() => setConfirming(null)}>
                        Cancel
                      </button>
                      <button
                        type="button"
                        className="is-danger"
                        onClick={() => void deleteTagNamed(t.tag).then(apply)}
                      >
                        Delete
                      </button>
                    </>
                  ) : (
                    <>
                      <span className="tag-mark" aria-hidden="true">
                        #
                      </span>
                      <span className="setting-item-label">{t.tag}</span>
                      <span className="sidebar-count">
                        {t.folders} folder{t.folders === 1 ? "" : "s"}
                      </span>
                      <button
                        type="button"
                        className="setting-delete"
                        aria-label={`Delete ${t.tag}`}
                        title={`Delete ${t.tag}`}
                        onClick={() => setConfirming(t.tag)}
                      >
                        <svg viewBox="0 0 16 16" aria-hidden="true">
                          <path
                            d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.25c.05.7.63 1.25 1.33 1.25h3.14c.7 0 1.28-.55 1.33-1.25l.6-8.25"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="1.3"
                            strokeLinecap="round"
                            strokeLinejoin="round"
                          />
                        </svg>
                      </button>
                    </>
                  )}
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </section>
  );
}
