import type { TagInfo } from "../model/tags";
import { tagColors, tagName } from "../presenter/tagPresenter";
import { DrivePicker } from "./DrivePicker";
import { TagMark } from "./TagMark";

type Props = {
  /** The drive the search covers. */
  active: string | null;
  /** The drive now active, and why switching failed if it did. */
  onActiveChange: (active: string | null, error: string | null) => void;
  /** Every tag that can be picked: the preset tags, then custom ones. */
  tags: TagInfo[];
  /** The tags the search is filtered by, shown as selected. */
  selectedTags: string[];
  /** Turns the filter by `tag` on or off. */
  onTag: (tag: string) => void;
};

/**
 * The sidebar, as in Finder: where the search looks (the drive), then the tags
 * folders carry. A heavier material than the content, so it reads as structure.
 */
export function Sidebar({ active, onActiveChange, tags, selectedTags, onTag }: Props) {
  const colors = tagColors(tags);
  return (
    <nav className="sidebar" aria-label="Sidebar">
      <section className="sidebar-section">
        <h2 className="sidebar-heading">Location</h2>
        <DrivePicker active={active} onChange={onActiveChange} />
      </section>
      <section className="sidebar-section is-tags">
        <h2 className="sidebar-heading">Tags</h2>
        <ul className="sidebar-list">
          {tags.map((t) => (
            <li key={t.tag}>
              <button
                type="button"
                className="sidebar-item"
                aria-pressed={selectedTags.includes(t.tag)}
                title={`Filter by the ${t.folders} folder${t.folders === 1 ? "" : "s"} tagged ${t.tag}`}
                onClick={() => onTag(t.tag)}
              >
                <TagMark tag={t.tag} colors={colors} />
                <span className="sidebar-label">{tagName(t.tag, colors)}</span>
                {t.folders > 0 && <span className="sidebar-count">{t.folders}</span>}
              </button>
            </li>
          ))}
        </ul>
        {tags.every((t) => t.preset) && (
          <p className="sidebar-empty">Tag a folder from its right-click menu, or make tags in Settings.</p>
        )}
      </section>
    </nav>
  );
}
