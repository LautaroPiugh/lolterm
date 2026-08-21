import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";
import "@xterm/xterm/css/xterm.css";
import { applyDocumentTheme, rememberedTheme } from "./themes";

applyDocumentTheme(rememberedTheme());

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
