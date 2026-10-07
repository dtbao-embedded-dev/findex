import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { SplashView } from "./view/SplashView";

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <SplashView />
    </StrictMode>,
  );
}
