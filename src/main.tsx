// SPDX-License-Identifier: GPL-3.0-or-later
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { installWebviewChrome } from "./lib/webviewChrome";
import "./index.css";

installWebviewChrome();

const container = document.getElementById("root");
if (!container) throw new Error("Missing #root element");

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

requestAnimationFrame(() => {
  requestAnimationFrame(() => {
    void revealWindow();
  });
});

async function revealWindow() {
  if (!("__TAURI_INTERNALS__" in window)) return;
  try {
    const { invoke } = await import("@tauri-apps/api/core");

    await invoke("ready_to_show");
  } catch {
  }
}
