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

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>
      <ModulesProvider>
        <App />
      </ModulesProvider>
    </I18nProvider>
  </React.StrictMode>,
);
