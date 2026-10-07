import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { SplashView } from "./view/SplashView";
import { blockWebMenu } from "./view/blockWebMenu";

blockWebMenu();

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <SplashView />
    </StrictMode>,
  );
}
