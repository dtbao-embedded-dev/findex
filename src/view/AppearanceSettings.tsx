import { THEMES, type ThemeId } from "../presenter/themePresenter";

type Props = {
  theme: ThemeId;
  onTheme: (theme: ThemeId) => void;
};

/**
 * The Appearance section of the settings: Findex is always dark, so the choice is
 * its colours. Each swatch carries its own theme, so it previews it whatever theme
 * is on; picking one applies it at once.
 */
export function AppearanceSettings({ theme, onTheme }: Props) {
  return (
    <section className="sheet-section" aria-labelledby="settings-appearance">
      <h3 id="settings-appearance" className="sheet-heading">
        Appearance
      </h3>
      <div className="setting-group">
        <div className="setting-row is-stacked">
          <div className="setting-text">
            <span className="setting-label">Theme</span>
            <p className="setting-hint">Findex is always dark; pick its colours.</p>
          </div>
          <div className="theme-grid" role="radiogroup" aria-label="Theme">
            {THEMES.map((t) => (
              <button
                key={t.id}
                type="button"
                role="radio"
                aria-checked={t.id === theme}
                className="theme-option"
                onClick={() => onTheme(t.id)}
              >
                <span className="theme-swatch" data-theme={t.id} aria-hidden="true">
                  <span className="theme-swatch-bar" />
                  <span className="theme-swatch-row" />
                  <span className="theme-swatch-dot" />
                </span>
                <span className="theme-name">{t.name}</span>
              </button>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
