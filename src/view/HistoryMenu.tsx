type Props = {
  /** Recent searches, newest first. */
  entries: string[];
  /** The entry the arrow keys are on; -1 for none. */
  active: number;
  onPick: (entry: string) => void;
  onForget: (entry: string) => void;
  onClear: () => void;
  onHover: (index: number) => void;
};

/**
 * Recent searches, dropped from the empty search field while it has focus. It never
 * takes focus itself (a press keeps it in the field), so typing simply carries on
 * and closes it; ↑/↓ and Enter pick from the field.
 */
export function HistoryMenu({ entries, active, onPick, onForget, onClear, onHover }: Props) {
  return (
    <div className="history-menu" onMouseDown={(event) => event.preventDefault()}>
      <div className="history-header">
        <span>Recent searches</span>
        <button type="button" className="history-clear" tabIndex={-1} onClick={onClear}>
          Clear
        </button>
      </div>
      <ul id="search-history" className="history-list" role="listbox" aria-label="Recent searches">
        {entries.map((entry, index) => (
          <li
            key={entry}
            id={`history-${index}`}
            role="option"
            aria-selected={index === active}
            className={index === active ? "history-item is-active" : "history-item"}
            onMouseEnter={() => onHover(index)}
            onClick={() => onPick(entry)}
          >
            <svg className="history-glyph" viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" strokeWidth="1.3" />
              <path
                d="M8 4.75V8l2.25 1.5"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.3"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            <span className="history-text">{entry}</span>
            <button
              type="button"
              className="history-forget"
              tabIndex={-1}
              aria-label={`Remove “${entry}” from recent searches`}
              title="Remove"
              onClick={(event) => {
                event.stopPropagation();
                onForget(entry);
              }}
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M5 5l6 6M11 5l-6 6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              </svg>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
