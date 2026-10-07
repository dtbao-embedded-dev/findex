import type { StatusLine } from "../presenter/statusPresenter";

/** The index state, one quiet line at the bottom; notes show in full on hover. */
export function StatusBar({ line }: { line: StatusLine }) {
  return (
    <footer className="statusbar" role="status">
      <span className={line.busy ? "status-dot is-busy" : "status-dot"} aria-hidden="true" />
      <span>{line.text}</span>
      {line.notes.length > 0 && (
        <span className="status-note" title={line.notes.join("\n")}>
          {line.notes[0]}
        </span>
      )}
    </footer>
  );
}
