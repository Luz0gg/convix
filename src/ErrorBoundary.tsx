// Last line of defence: a rendering error shows this screen (and lands in the log)
// instead of leaving a blank window. Works even if i18n itself is what broke.
import { Component, type ErrorInfo, type ReactNode } from "react";
import { initialLang } from "./i18n";
import { api } from "./lib/api";

const TEXT = {
  fr: { title: "Oups, l'interface a rencontré un problème", body: "Tes fichiers ne sont pas touchés. Le détail est dans le journal d'erreurs.", reload: "Recharger" },
  en: { title: "Oops, the interface ran into a problem", body: "Your files are untouched. Details are in the error log.", reload: "Reload" },
};

export class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    api.logUi("error", `render: ${error.message} ${info.componentStack?.split("\n").slice(0, 4).join(" ") ?? ""}`).catch(() => {});
  }

  render() {
    if (!this.state.error) return this.props.children;
    const text = TEXT[initialLang()] ?? TEXT.fr;
    return (
      <div role="alert" className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
        <h1 className="text-lg font-bold">{text.title}</h1>
        <p className="max-w-md text-sm text-muted">{text.body}</p>
        <p className="selectable max-w-md text-xs text-danger">{this.state.error.message}</p>
        <button onClick={() => location.reload()} className="rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-on-accent">
          {text.reload}
        </button>
      </div>
    );
  }
}
