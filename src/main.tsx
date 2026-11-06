import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ModulesProvider } from "./features/modules/modules";
import { I18nProvider } from "./i18n";
import "./styles.css";

// In a plain browser during development, fake the Rust side (see src/dev/mockTauri.ts).
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
  await import("./dev/mockTauri");
}

// Anything that goes wrong in the interface lands in the local log (Réglages › À propos).
const report = (message: string) => import("./lib/api").then(({ api }) => api.logUi("error", message)).catch(() => {});
window.addEventListener("error", (e) => report(`${e.message} (${e.filename}:${e.lineno})`));
window.addEventListener("unhandledrejection", (e) => report(`unhandled: ${String(e.reason)}`));
window.addEventListener("securitypolicyviolation", (e) => report(`CSP blocked ${e.blockedURI} (${e.violatedDirective})`));

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>
      <ModulesProvider>
        <App />
      </ModulesProvider>
    </I18nProvider>
  </React.StrictMode>,
);

import("./lib/api").then(({ api }) => api.logUi("info", "interface ready")).catch(() => {});
