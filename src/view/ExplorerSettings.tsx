import { useEffect, useState } from "react";

import {
  loadExplorerMenu,
  setExplorerMenuTo,
  syncMenu,
  type MenuSetting,
} from "../presenter/explorerPresenter";

/**
 * The Explorer section of the settings: one switch for "Tag with Findex" in
 * Explorer's right-click menu, which tags a folder while Findex stays in the tray.
 * Opening the settings checks the menu is really in place, so a missing helper
 * shows here rather than as a menu entry that does nothing.
 */
export function ExplorerSettings() {
  const [setting, setSetting] = useState<MenuSetting | null>(null);

  useEffect(() => {
    void loadExplorerMenu().then(async (loaded) => {
      const error = loaded.enabled && loaded.error === null ? await syncMenu() : loaded.error;
      setSetting({ ...loaded, error });
    });
  }, []);

  const toggle = (enabled: boolean) => {
    void setExplorerMenuTo(enabled, setting?.enabled ?? !enabled).then(setSetting);
  };

  return (
    <section className="sheet-section" aria-labelledby="settings-explorer">
      <h3 id="settings-explorer" className="sheet-heading">
        Explorer
      </h3>
      <div className="setting-group">
        <div className="setting-row">
          <div className="setting-text">
            <label htmlFor="explorer-menu" className="setting-label">
              Tag folders from Explorer
            </label>
            <p className="setting-hint">
              Adds “Tag with Findex” to a folder’s right-click menu, so a folder can be tagged
              while Findex stays in the tray. On Windows 11 it is under “Show more options”.
            </p>
          </div>
          <input
            id="explorer-menu"
            type="checkbox"
            role="switch"
            className="switch"
            disabled={setting === null}
            checked={setting?.enabled ?? false}
            onChange={(event) => toggle(event.target.checked)}
          />
        </div>
      </div>
      {setting?.error && (
        <p className="error setting-error" role="alert">
          {setting.error}
        </p>
      )}
    </section>
  );
}
