import { useI18n } from "../../i18n";
import { formatSize } from "../../lib/api";
import type { ModuleProgress } from "../../lib/types";

export function ModuleProgressBar({ progress }: { progress: ModuleProgress }) {
  const { t } = useI18n();
  const fraction = progress.phase === "downloading" && progress.total > 0 ? progress.done / progress.total : null;
  return (
    <div className="space-y-1">
      <div className="h-2 overflow-hidden rounded-full bg-surface-2">
        <div
          className={`h-full rounded-full bg-accent transition-[width] duration-200 ${fraction === null ? "w-full animate-pulse" : ""}`}
          style={fraction === null ? undefined : { width: `${Math.round(fraction * 100)}%` }}
        />
      </div>
      <p className="text-xs text-muted">
        {t(`module.phase.${progress.phase}`)}
        {fraction !== null && ` · ${formatSize(progress.done)} / ${formatSize(progress.total)}`}
      </p>
    </div>
  );
}
