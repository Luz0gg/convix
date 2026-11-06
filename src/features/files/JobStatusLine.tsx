import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useI18n } from "../../i18n";
import { fileName, formatSize } from "../../lib/api";
import type { JobEvent } from "../../lib/types";

export function JobStatusLine({ job }: { job: JobEvent }) {
  const { t } = useI18n();

  if (job.status === "running") {
    return <p className="mt-3 animate-pulse text-sm text-accent">{t("job.running")}</p>;
  }
  if (job.status === "cancelled") {
    return <p className="mt-3 text-sm text-muted">{t("job.cancelled")}</p>;
  }
  if (job.status === "failed") {
    return <p className="mt-3 text-sm text-danger">{t("job.failed", { error: job.error ?? "?" })}</p>;
  }

  const out = job.outputs[0];
  const delta = job.inputSize > 0 ? Math.round(((job.outputSize - job.inputSize) / job.inputSize) * 100) : 0;
  return (
    <div className="mt-3 flex items-center gap-2 rounded-lg bg-success/10 px-3 py-2 text-sm">
      <span className="text-success">✓</span>
      <span className="min-w-0 flex-1 truncate" title={out}>{out ? fileName(out) : ""}</span>
      <span className="shrink-0 text-xs text-muted">
        {formatSize(job.inputSize)} → {formatSize(job.outputSize)}
      </span>
      <span className={`shrink-0 text-xs font-semibold ${delta <= 0 ? "text-success" : "text-amber-500"}`}>
        {delta > 0 ? "+" : delta < 0 ? "−" : "±"}
        {Math.abs(delta)} %
      </span>
      {out && (
        <button onClick={() => revealItemInDir(out)} className="shrink-0 rounded-md border border-border px-2 py-0.5 text-xs hover:border-accent hover:text-accent">
          {t("job.reveal")}
        </button>
      )}
    </div>
  );
}
