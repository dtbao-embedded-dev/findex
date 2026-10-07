import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./view/App";
import { blockWebMenu } from "./view/blockWebMenu";

blockWebMenu();

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
