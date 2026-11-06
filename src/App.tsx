import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useI18n } from "./i18n";
import { api, dirName, ext, rememberedOptions } from "./lib/api";
import { supports } from "./lib/ops";
import type { FileInfo, JobEvent, LaunchRequest, OperationDescriptor, OptionValues, Settings } from "./lib/types";
import { DropZone } from "./features/drop/DropZone";
import { useFileDrop } from "./features/drop/useFileDrop";
import { BatchBar } from "./features/files/BatchBar";
import { FileCard } from "./features/files/FileCard";
import { SettingsPanel } from "./features/settings/SettingsPanel";
import { ModulePrompt } from "./features/modules/ModulePrompt";
import { useModules } from "./features/modules/modules";
import { UpdateBanner } from "./features/updates/UpdateBanner";
import type { Update } from "@tauri-apps/plugin-updater";

type Toast = { kind: "ok" | "error"; text: string };
/** An action waiting for a module download (FFmpeg…). */
type PendingRun = { moduleId: string; op: OperationDescriptor; targets: FileInfo[]; options: OptionValues; combine: boolean };
type RunFlags = { moduleJustInstalled?: boolean; /** Several files → one result, shown in the batch bar. */ combine?: boolean };
/** Which options panel is open: on one file (its path) or on the batch bar. */
type Selection = { scope: string; op: OperationDescriptor } | null;
const BATCH = "__batch__";

export default function App() {
  const { t, setLang } = useI18n();
  const [files, setFiles] = useState<FileInfo[]>([]);
  const [jobs, setJobs] = useState<Record<string, JobEvent>>({});
  const [selection, setSelection] = useState<Selection>(null);
  const [running, setRunning] = useState<string[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [toast, setToast] = useState<Toast | null>(null);
  const [pendingRun, setPendingRun] = useState<PendingRun | null>(null);
  const [combineJob, setCombineJob] = useState<JobEvent | undefined>();
  const combineBatches = useRef(new Set<string>());
  const [update, setUpdate] = useState<Update | null>(null);
  const { modules, missingFor } = useModules();
  const settingsRef = useRef<Settings | null>(null);
  settingsRef.current = settings;

  // ---- settings: Rust is the source of truth ----
  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      setLang(s.language);
    });
  }, [setLang]);

  // ---- new version? (a few seconds after start, never blocking) ----
  const checkUpdate = useCallback(async () => {
    try {
      const found = await api.checkUpdate();
      setUpdate(found);
      return found ? "found" : "upToDate";
    } catch (e) {
      api.logUi("error", `update check: ${e}`).catch(() => {});
      return "error";
    }
  }, []);
  useEffect(() => {
    if (!settings?.checkUpdates) return;
    const id = setTimeout(checkUpdate, 4000);
    return () => clearTimeout(id);
  }, [settings?.checkUpdates, checkUpdate]);

  useEffect(() => {
    const root = document.documentElement;
    if (!settings || settings.theme === "system") delete root.dataset.theme;
    else root.dataset.theme = settings.theme;
  }, [settings?.theme]);

  const changeSettings = async (next: Settings) => {
    setSettings(next);
    setLang(next.language);
    try {
      setSettings(await api.updateSettings(next));
    } catch (e) {
      setToast({ kind: "error", text: String(e) });
    }
  };

  // ---- toasts disappear by themselves ----
  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(null), 6000);
    return () => clearTimeout(id);
  }, [toast]);

  // ---- per-file progress ----
  useEffect(() => {
    const off = listen<JobEvent>("job-event", (e) => {
      if (combineBatches.current.has(e.payload.batchId)) setCombineJob(e.payload);
      else setJobs((prev) => ({ ...prev, [e.payload.input]: e.payload }));
    });
    return () => {
      off.then((fn) => fn());
    };
  }, []);

  const addPaths = useCallback(async (paths: string[]) => {
    const infos = await api.inspectFiles(paths);
    setFiles((prev) => {
      const known = new Set(prev.map((f) => f.path));
      return [...prev, ...infos.filter((f) => !known.has(f.path))];
    });
    return infos;
  }, []);

  const { hovering } = useFileDrop(addPaths);

  // ---- run an operation on one or many files ----
  const runOp = useCallback(
    async (op: OperationDescriptor, targets: FileInfo[], options: OptionValues, { moduleJustInstalled = false, combine = false }: RunFlags = {}) => {
      const compatible = targets.filter((f) => supports(f, op.id));
      if (compatible.length === 0) return;

      // Needs a module first (audio → FFmpeg): offer the download, then run.
      const missing = missingFor(op)[0];
      if (missing && !moduleJustInstalled) {
        setSelection(null);
        setPendingRun({ moduleId: missing.id, op, targets: compatible, options, combine });
        return;
      }

      let outputDir: string | null = null;
      let outputPath: string | null = null;
      if (settingsRef.current?.output.mode === "askEachTime") {
        if (compatible.length === 1 || combine) {
          const f = compatible[0]; // a combined result is named after the first file
          const sameFormat = op.output === f.format;
          const outExt = sameFormat ? (f.name.split(".").pop() ?? "") : ext(op.output ?? "");
          const stem = f.name.replace(/\.[^.]+$/, "");
          outputPath = await save({
            defaultPath: `${dirName(f.path)}\\${stem}${op.outputSuffix ?? ""}.${outExt}`,
            filters: [{ name: outExt.toUpperCase(), extensions: [outExt] }],
          });
          if (!outputPath) return;
        } else {
          const dir = await open({ directory: true });
          if (typeof dir !== "string") return;
          outputDir = dir;
        }
      }

      const batchId = crypto.randomUUID();
      if (combine) {
        combineBatches.current.add(batchId);
        setCombineJob(undefined);
      }
      setSelection(null);
      setRunning((r) => [...r, batchId]);
      try {
        const results = await api.runBatch({
          batchId,
          operationId: op.id,
          inputs: compatible.map((f) => f.path),
          options,
          outputDir,
          outputPath,
        });
        if (combine && results[0]) setCombineJob(results[0]);
        const done = results.filter((r) => r.status === "done").length;
        const failed = results.filter((r) => r.status === "failed").length;
        if (results.length > 1 && !combine) setToast({ kind: failed ? "error" : "ok", text: t("batch.summary", { done, failed }) });
      } catch (e) {
        setToast({ kind: "error", text: String(e) });
      } finally {
        setRunning((r) => r.filter((id) => id !== batchId));
      }
    },
    [t, missingFor],
  );

  // ---- Explorer context menu: files (and maybe an operation) passed at launch ----
  useEffect(() => {
    const handle = async (req: LaunchRequest | null) => {
      if (!req) return;
      const infos = await addPaths(req.files);
      if (!req.operation) return;
      const op = infos.flatMap((f) => f.operations).find((o) => o.id === req.operation);
      if (op) runOp(op, infos, rememberedOptions(op.id, op.options));
    };
    api.takeLaunchRequest().then(handle);
    const off = listen<LaunchRequest>("launch-request", (e) => handle(e.payload));
    return () => {
      off.then((fn) => fn());
    };
  }, [addPaths, runOp]);

  const removeFile = (path: string) => {
    setFiles((prev) => prev.filter((f) => f.path !== path));
    setJobs(({ [path]: _, ...rest }) => rest);
  };
  const clearDone = () => {
    const done = new Set(Object.values(jobs).filter((j) => j.status === "done").map((j) => j.input));
    setFiles((prev) => prev.filter((f) => !done.has(f.path)));
  };
  const hasDone = Object.values(jobs).some((j) => j.status === "done" && files.some((f) => f.path === j.input));

  return (
    <div className="flex h-full flex-col gap-4 p-5">
      <header className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <img src="/icon.svg" alt="" className="h-10 w-10" draggable={false} />
          <div>
            <h1 className="text-xl font-bold">
              Conv<span className="text-accent">ix</span>
            </h1>
            <p className="text-sm text-muted">{t("app.tagline")}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {running.length > 0 && (
            <button
              onClick={() => running.forEach((id) => api.cancelBatch(id))}
              className="rounded-lg border border-danger px-3 py-1.5 text-sm text-danger hover:bg-danger/10"
            >
              {t("run.cancel")}
            </button>
          )}
          {hasDone && (
            <button onClick={clearDone} className="rounded-lg px-3 py-1.5 text-sm text-muted hover:bg-surface-2">
              {t("files.clearDone")}
            </button>
          )}
          {files.length > 0 && (
            <button onClick={() => (setFiles([]), setJobs({}), setCombineJob(undefined))} className="rounded-lg px-3 py-1.5 text-sm text-muted hover:bg-surface-2">
              {t("files.clear")}
            </button>
          )}
          <button
            onClick={() => setShowSettings(true)}
            className="rounded-lg border border-border px-3 py-1.5 text-sm hover:bg-surface-2"
            aria-label={t("app.settings")}
          >
            ⚙ {t("app.settings")}
          </button>
        </div>
      </header>

      {update && <UpdateBanner update={update} onDismiss={() => setUpdate(null)} />}

      <DropZone hovering={hovering} compact={files.length > 0} />

      {files.length > 0 && (
        <section className="flex-1 space-y-3 overflow-y-auto pr-1">
          {files.length > 1 && (
            <BatchBar
              files={files}
              selectedOp={selection?.scope === BATCH ? selection.op : null}
              onSelect={(op) => setSelection(op ? { scope: BATCH, op } : null)}
              onRun={(op, values, ordered) => runOp(op, ordered ?? files, values, { combine: Boolean(ordered) })}
              combineJob={combineJob}
            />
          )}
          {files.map((f) => (
            <FileCard
              key={f.path}
              file={f}
              job={jobs[f.path]}
              selectedOp={selection?.scope === f.path ? selection.op : null}
              onSelect={(op) => setSelection(op ? { scope: f.path, op } : null)}
              onRun={(op, values) => runOp(op, [f], values)}
              onRemove={() => removeFile(f.path)}
            />
          ))}
        </section>
      )}

      {showSettings && settings && <SettingsPanel settings={settings} onChange={changeSettings} onClose={() => setShowSettings(false)} onCheckUpdate={checkUpdate} />}

      {pendingRun && modules.find((m) => m.id === pendingRun.moduleId) && (
        <ModulePrompt
          module={modules.find((m) => m.id === pendingRun.moduleId)!}
          onClose={() => setPendingRun(null)}
          onInstalled={() => {
            const { op, targets, options, combine } = pendingRun;
            setPendingRun(null);
            runOp(op, targets, options, { moduleJustInstalled: true, combine });
          }}
        />
      )}

      {toast && (
        <div
          onClick={() => setToast(null)}
          className={`fixed bottom-4 left-1/2 z-30 max-w-[90%] -translate-x-1/2 cursor-pointer rounded-lg px-4 py-2 text-sm shadow-lg ${
            toast.kind === "ok" ? "bg-success text-black" : "bg-danger text-white"
          }`}
        >
          {toast.text}
        </div>
      )}
    </div>
  );
}
