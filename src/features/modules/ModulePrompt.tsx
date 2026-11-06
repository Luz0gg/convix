// Shown when an action needs a module that isn't installed yet (e.g. audio → FFmpeg).
// After the download, the action the user asked for runs by itself.
import { useState } from "react";
import { useI18n } from "../../i18n";
import { errorText, formatSize, isCancelled } from "../../lib/api";
import type { ModuleInfo } from "../../lib/types";
import { ModuleProgressBar } from "./ModuleProgressBar";
import { useModules } from "./modules";

export function ModulePrompt({ module, onInstalled, onClose }: { module: ModuleInfo; onInstalled: () => void; onClose: () => void }) {
  const { t } = useI18n();
  const { install, cancel, progress } = useModules();
  const [error, setError] = useState<string | null>(null);
  const current = progress[module.id];

  const start = async () => {
    setError(null);
    try {
      await install(module.id);
      onInstalled();
    } catch (e) {
      if (!isCancelled(e)) setError(errorText(t, e));
    }
  };

  return (
    <div className="fixed inset-0 z-30 flex items-center justify-center bg-black/50 p-6">
      <div className="w-full max-w-md space-y-4 rounded-xl border border-border bg-surface p-6 shadow-xl">
        <h2 className="text-lg font-bold">{t("module.prompt.title", { module: module.name })}</h2>
        <p className="text-sm text-muted">{t(`module.${module.id}.why`)}</p>
        <ul className="space-y-1 text-sm text-muted">
          <li>• {t("module.prompt.once", { size: formatSize(module.downloadSize) })}</li>
          <li>• {t("module.prompt.verified")}</li>
          <li>• {t("module.prompt.license", { license: module.license })}</li>
        </ul>

        {current && <ModuleProgressBar progress={current} />}
        {error && <p className="text-sm text-danger">{error}</p>}

        <div className="flex justify-end gap-2">
          {current ? (
            <button onClick={() => cancel(module.id)} className="rounded-lg border border-danger px-3 py-1.5 text-sm text-danger hover:bg-danger/10">
              {t("run.cancel")}
            </button>
          ) : (
            <>
              <button onClick={onClose} className="rounded-lg px-3 py-1.5 text-sm text-muted hover:bg-surface-2">
                {t("run.close")}
              </button>
              <button onClick={start} className="rounded-lg bg-accent px-4 py-1.5 text-sm font-medium text-white hover:opacity-90">
                {error ? t("module.prompt.retry") : t("module.prompt.download", { size: formatSize(module.downloadSize) })}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
