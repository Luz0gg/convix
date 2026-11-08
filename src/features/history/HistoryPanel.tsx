// "What did I convert, and where is it?" — the last conversions, with a button to show each result.
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { useI18n } from "../../i18n";
import { fileName } from "../../lib/api";
import { clearHistory, loadHistory, onHistoryChange, type HistoryEntry } from "../../lib/history";
import { Dialog } from "../ui/Dialog";

export function HistoryPanel({ onClose }: { onClose: () => void }) {
  const { t, lang } = useI18n();
  const [entries, setEntries] = useState<HistoryEntry[]>(loadHistory);
  useEffect(() => onHistoryChange(setEntries), []);
  const when = new Intl.DateTimeFormat(lang, { dateStyle: "short", timeStyle: "short" });

  return (
    <Dialog title={t("history.title")} onClose={onClose} variant="drawer">
      <header className="flex items-center justify-between">
        <h2 className="text-lg font-bold">{t("history.title")}</h2>
        <button onClick={onClose} className="rounded-md px-2 py-1 text-muted hover:bg-surface-2" aria-label={t("history.close")} title={t("history.close")}>
          ✕
        </button>
      </header>

      {entries.length === 0 ? (
        <p className="text-sm text-muted">{t("history.empty")}</p>
      ) : (
        <>
          <ol className="space-y-2">
            {entries.map((e) => (
              <li key={`${e.at}-${e.operationId}`} className="rounded-lg border border-border p-3">
                <div className="flex items-baseline justify-between gap-3">
                  <span className="text-sm font-medium">{e.label}</span>
                  <time className="shrink-0 text-xs text-muted" dateTime={new Date(e.at).toISOString()}>
                    {when.format(e.at)}
                  </time>
                </div>
                <div className="mt-1 flex items-center gap-2 text-xs">
                  <span className="selectable min-w-0 flex-1 truncate text-muted" title={e.outputs.join("\n")}>
                    {e.outputs[0] ? fileName(e.outputs[0]) : ""}
                    {e.outputs.length > 1 && ` ${t("job.moreFiles", { count: e.outputs.length - 1 })}`}
                  </span>
                  {e.failed > 0 && <span className="shrink-0 text-danger">{t("history.failed", { count: e.failed })}</span>}
                  {e.outputs[0] && (
                    <button
                      onClick={() => revealItemInDir(e.outputs[0]).catch(() => {})}
                      className="shrink-0 rounded-md border border-border px-2 py-0.5 hover:border-accent hover:text-accent"
                    >
                      {t("history.open")}
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ol>
          <button onClick={clearHistory} className="text-xs text-muted hover:text-danger">
            {t("history.clear")}
          </button>
        </>
      )}
    </Dialog>
  );
}
