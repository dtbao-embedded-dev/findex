import { useEffect, useState } from "react";

import {
  canContinue,
  describeDrive,
  initialSetup,
  loadSetup,
  saveSetup,
  toggleDrive,
} from "../presenter/setupPresenter";

type Props = {
  /** First run has no Cancel: there is nothing to go back to. */
  firstRun: boolean;
  onDone: () => void;
  onCancel: () => void;
};

/** Pick the drives to index. Each drive is a card that toggles on press. */
export function SetupView({ firstRun, onDone, onCancel }: Props) {
  const [state, setState] = useState(initialSetup);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    void loadSetup().then((loaded) => {
      setState(loaded);
      setLoading(false);
    });
  }, []);

  const submit = () => {
    setState((s) => ({ ...s, saving: true }));
    void saveSetup(state).then((result) => {
      setState(result.state);
      if (result.saved) {
        onDone();
      }
    });
  };

  return (
    <section className="setup" aria-busy={loading}>
      <header className="setup-header">
        <h1>Choose drives to index</h1>
        <p className="setup-lede">
          Findex reads each drive&rsquo;s file list once, then keeps it current. You can
          change this later from <strong>Drives</strong>.
        </p>
      </header>

      <ul className="drive-list">
        {state.drives.map((drive) => {
          const row = describeDrive(drive);
          const checked = state.selected.includes(drive.letter);
          return (
            <li key={drive.letter}>
              <label className={checked ? "drive-card is-selected" : "drive-card"}>
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={() => setState((s) => toggleDrive(s, drive.letter))}
                />
                <DriveGlyph removable={drive.removable} />
                <span className="drive-text">
                  <span className="drive-title">{row.title}</span>
                  <span className="drive-detail">{row.detail}</span>
                  <span className="drive-meter" aria-hidden="true">
                    <span style={{ width: `${row.usedPercent}%` }} />
                  </span>
                </span>
                <span className="drive-badge" data-method={drive.method}>
                  {row.method}
                </span>
                <span className="drive-check" aria-hidden="true" />
              </label>
            </li>
          );
        })}
      </ul>

      <p role="alert" className="error setup-error">
        {state.error}
      </p>

      <footer className="setup-footer">
        {!firstRun && (
          <button type="button" className="button-secondary" onClick={onCancel}>
            Cancel
          </button>
        )}
        <button type="button" disabled={loading || !canContinue(state)} onClick={submit}>
          {firstRun ? "Start indexing" : "Save"}
        </button>
      </footer>
    </section>
  );
}

/** A drive, drawn rather than loaded, so it follows the text colour. */
function DriveGlyph({ removable }: { removable: boolean }) {
  return (
    <svg className="drive-glyph" viewBox="0 0 32 32" aria-hidden="true">
      {removable ? (
        <>
          <rect x="9" y="3" width="14" height="8" rx="1.5" fill="none" stroke="currentColor" strokeWidth="2" />
          <rect x="6" y="10" width="20" height="19" rx="4" fill="currentColor" opacity="0.18" stroke="currentColor" strokeWidth="2" />
        </>
      ) : (
        <>
          <rect x="3" y="8" width="26" height="16" rx="4" fill="currentColor" opacity="0.18" stroke="currentColor" strokeWidth="2" />
          <circle cx="23" cy="16" r="1.75" fill="currentColor" />
          <path d="M8 16h8" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </>
      )}
    </svg>
  );
}
