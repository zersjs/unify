import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles/tokens.css";
import "./styles/landing.css";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root container #root is missing from index.html");
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
