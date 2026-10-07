import type { TagCount } from "../model/tags";
import { DrivePicker } from "./DrivePicker";

type Props = {
  /** The drive the search covers. */
  active: string | null;
  /** The drive now active, and why switching failed if it did. */
  onActiveChange: (active: string | null, error: string | null) => void;
  /** The tags in use, most used first. */
  tags: TagCount[];
  onTag: (tag: string) => void;
};

/**
 * The sidebar, as in Finder: where the search looks (the drive), then the tags
 * folders carry. A heavier material than the content, so it reads as structure.
 */
export function Sidebar({ active, onActiveChange, tags, onTag }: Props) {
  return (
    <nav className="sidebar" aria-label="Sidebar">
      <section className="sidebar-section">
        <h2 className="sidebar-heading">Location</h2>
        <DrivePicker active={active} onChange={onActiveChange} />
      </section>
      <section className="sidebar-section is-tags">
        <h2 className="sidebar-heading">Tags</h2>
        {tags.length === 0 ? (
          <p className="sidebar-empty">Right-click a folder and choose Tags… to tag it.</p>
        ) : (
          <ul className="sidebar-list">
            {tags.map((t) => (
              <li key={t.tag}>
                <button
                  type="button"
                  className="sidebar-item"
                  title={`Show the ${t.folders} folder${t.folders === 1 ? "" : "s"} tagged ${t.tag}`}
                  onClick={() => onTag(t.tag)}
                >
                  <span className="tag-mark" aria-hidden="true">
                    #
                  </span>
                  <span className="sidebar-label">{t.tag}</span>
                  <span className="sidebar-count">{t.folders}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </nav>
  );
}
