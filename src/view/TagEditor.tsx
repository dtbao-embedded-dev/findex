import { useEffect, useLayoutEffect, useRef, useState } from "react";

import type { Hit } from "../model/search";
import type { TagInfo } from "../model/tags";
import { placeMenu } from "../presenter/menuPresenter";
import {
  addTagTo,
  removeTagFrom,
  suggestTags,
  tagColors,
  tagName,
} from "../presenter/tagPresenter";
import { TagMark } from "./TagMark";
import { useDismiss } from "./useDismiss";

type Props = {
  /** Where the menu that opened it was. */
  at: { x: number; y: number };
  folder: Hit;
  tags: string[];
  /** Every tag that can be picked. */
  catalog: TagInfo[];
  /** The folder's tags after each change. */
  onChange: (tags: string[]) => void;
  onClose: () => void;
};

/**
 * A small panel to tag one folder: the preset tags (Working, Doc, …) as pills to toggle, its
 * custom tags as chips to remove, a field to add one (a new name becomes a custom
 * tag), and the custom tags already made as one-press suggestions. Each change is
 * saved at once.
 */
export function TagEditor({ at, folder, tags, catalog, onChange, onClose }: Props) {
  const panel = useRef<HTMLDivElement>(null);
  const field = useRef<HTMLInputElement>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [place, setPlace] = useState({ left: at.x, top: at.y, origin: "top left" });

  useLayoutEffect(() => {
    const box = panel.current?.getBoundingClientRect();
    if (box) {
      setPlace(placeMenu(at, box, { width: window.innerWidth, height: window.innerHeight }));
    }
  }, [at]);

  useDismiss(panel, onClose);

  useEffect(() => {
    field.current?.focus();
  }, []);

  const apply = (edit: { tags: string[] | null; error: string | null }) => {
    setError(edit.error);
    if (edit.tags !== null) {
      onChange(edit.tags);
      setText("");
    }
    field.current?.focus();
  };

  const colors = tagColors(catalog);
  const presets = catalog.filter((t) => t.preset);
  const custom = tags.filter((tag) => !(tag in colors));
  const suggestions = suggestTags(catalog, tags, text);

  return (
    <div
      ref={panel}
      className="tag-editor"
      role="dialog"
      aria-label={`Tags for ${folder.name}`}
      style={{ left: place.left, top: place.top, transformOrigin: place.origin }}
    >
      <p className="tag-editor-title">
        Tags for <strong>{folder.name}</strong>
      </p>
      <div className="tag-colors" role="group" aria-label="Preset tags">
        {presets.map((t) => {
          const on = tags.includes(t.tag);
          return (
            <button
              key={t.tag}
              type="button"
              className="tag-color"
              aria-pressed={on}
              onClick={() =>
                void (on ? removeTagFrom(folder.path, t.tag) : addTagTo(folder.path, t.tag)).then(apply)
              }
            >
              <span className="tag-dot" data-color={t.color ?? undefined} aria-hidden="true" />
              {tagName(t.tag, colors)}
            </button>
          );
        })}
      </div>
      <div className="tag-row">
        {custom.length === 0 && <span className="tag-none">No tags of your own yet</span>}
        {custom.map((tag) => (
          <span key={tag} className="tag-chip">
            #{tag}
            <button
              type="button"
              aria-label={`Remove ${tag}`}
              onClick={() => void removeTagFrom(folder.path, tag).then(apply)}
            >
              ×
            </button>
          </span>
        ))}
      </div>
      <input
        ref={field}
        type="text"
        spellCheck={false}
        autoComplete="off"
        placeholder="Add a tag and press Enter"
        aria-label="New tag"
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            void addTagTo(folder.path, text).then(apply);
          }
        }}
      />
      {suggestions.length > 0 && (
        <div className="tag-row">
          {suggestions.map((tag) => (
            <button
              key={tag}
              type="button"
              className="tag-chip is-suggestion"
              onClick={() => void addTagTo(folder.path, tag).then(apply)}
            >
              <TagMark tag={tag} colors={colors} />
              {tag}
            </button>
          ))}
        </div>
      )}
      {error !== null && <p className="error tag-error">{error}</p>}
    </div>
  );
}
