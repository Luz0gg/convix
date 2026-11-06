// "Convix 1.1 is available": download, verify (signature), install, restart.
import type { Update } from "@tauri-apps/plugin-updater";
import { useState } from "react";
import { useI18n } from "../../i18n";
import { api, formatSize } from "../../lib/api";

export function UpdateBanner({ update, busy, onDismiss }: { update: Update; /** Conversions running. */ busy: boolean; onDismiss: () => void }) {
  const { t } = useI18n();
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const install = async () => {
    setError(null);
    setProgress({ done: 0, total: 0 });
    try {
      let done = 0;
      await update.download((e) => {
        if (e.event === "Started") setProgress({ done: 0, total: e.data.contentLength ?? 0 });
        if (e.event === "Progress") {
          done += e.data.chunkLength;
          setProgress((p) => ({ done, total: p?.total ?? 0 }));
        }
      });
      // On Windows the installer takes over: the app closes right away (no "exit" line in the log).
      await api.logUi("info", `installing update ${update.currentVersion} → ${update.version}`).catch(() => {});
      await update.install();
    } catch (e) {
      setProgress(null);
      setError(String(e));
      api.logUi("error", `update install: ${e}`).catch(() => {});
    }
  };

  return (
    <div role="status" className="flex flex-wrap items-center gap-3 rounded-xl border border-accent/40 bg-accent-soft px-4 py-2 text-sm">
      <span className="font-medium text-accent">{t("update.available", { version: update.version })}</span>
      {progress ? (
        <span className="text-xs text-muted">
          {t("update.downloading")}
          {progress.total > 0 && ` · ${formatSize(progress.done)} / ${formatSize(progress.total)}`}
        </span>
      ) : (
        <>
          <button
            onClick={install}
            disabled={busy}
            title={busy ? t("update.busy") : t("update.closing")}
            className="rounded-md bg-accent px-3 py-1 text-xs font-semibold text-on-accent hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {t("update.install")}
          </button>
          <button onClick={onDismiss} className="text-xs text-muted hover:text-text">
            {t("update.later")}
          </button>
          <span className="w-full text-xs text-muted">{busy ? t("update.busy") : t("update.closing")}</span>
        </>
      )}
      {error && <span className="selectable w-full text-xs text-danger">{error}</span>}
    </div>
  );
}
