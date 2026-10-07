import { useEffect, useLayoutEffect, useRef, useState } from "react";

import type { Hit } from "../model/search";
import type { TagCount } from "../model/tags";
import { placeMenu } from "../presenter/menuPresenter";
import { addTagTo, loadTags, removeTagFrom, suggestTags } from "../presenter/tagPresenter";
import { useDismiss } from "./useDismiss";

type Props = {
  /** Where the menu that opened it was. */
  at: { x: number; y: number };
  folder: Hit;
  tags: string[];
  /** The folder's tags after each change. */
  onChange: (tags: string[]) => void;
  onClose: () => void;
};

/**
 * A small panel to tag one folder: its tags as chips to remove, a field to add one,
 * and the tags already in use as one-press suggestions. Each change is saved at once.
 */
export function TagEditor({ at, folder, tags, onChange, onClose }: Props) {
  const panel = useRef<HTMLDivElement>(null);
  const field = useRef<HTMLInputElement>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [inUse, setInUse] = useState<TagCount[]>([]);
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
    void loadTags().then(setInUse);
  }, []);

  const apply = (edit: { tags: string[] | null; error: string | null }) => {
    setError(edit.error);
    if (edit.tags !== null) {
      onChange(edit.tags);
      setText("");
    }
    field.current?.focus();
  };

  const suggestions = suggestTags(inUse, tags, text);

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
      <div className="tag-row">
        {tags.length === 0 && <span className="tag-none">No tags yet</span>}
        {tags.map((tag) => (
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
              #{tag}
            </button>
          ))}
        </div>
      )}
      {error !== null && <p className="error tag-error">{error}</p>}
    </div>
  );
}
