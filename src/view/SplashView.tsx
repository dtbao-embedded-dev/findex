import { useEffect, useState } from "react";

import iconUrl from "../../src-tauri/icons/icon.svg";
import { initialSplash, loadSplash } from "../presenter/splashPresenter";
import { loadTheme } from "../presenter/themePresenter";

/** The splash window's whole content: icon, name, version and copyright. */
export function SplashView() {
  const [state, setState] = useState(initialSplash);

  useEffect(() => {
    void loadSplash().then(setState);
    // The splash wears the chosen theme too, so the main window does not change
    // colour as it takes over.
    void loadTheme().then((theme) => {
      document.documentElement.dataset.theme = theme;
    });
  }, []);

  return (
    <div className="splash">
      <img className="splash-icon" src={iconUrl} alt="" width={96} height={96} />
      <h1 className="splash-name">{state.name}</h1>
      <p className="splash-version">{state.version}</p>
      <p className="splash-copyright">{state.copyright}</p>
    </div>
  );
}
