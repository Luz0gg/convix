import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { useI18n } from "./i18n";
import { api, dirName, errorText, ext, rememberedOptions } from "./lib/api";
import { supports } from "./lib/ops";
import type { FileInfo, JobEvent, LaunchRequest, OperationDescriptor, OptionValues, Settings } from "./lib/types";
import { DropZone } from "./features/drop/DropZone";
import { useFileDrop } from "./features/drop/useFileDrop";
import { BatchBar } from "./features/files/BatchBar";
import { FileCard } from "./features/files/FileCard";
import { releaseThumbnails } from "./features/files/Thumbnail";
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

/** Used if the settings can't be read (the app stays usable). */
const DEFAULT_SETTINGS: Settings = {
  output: { mode: "nextToSource" },
  language: "fr",
  theme: "system",
  shellIntegration: true,
  quietContextMenu: true,
  checkUpdates: true,
};

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
  const [update, setUpdate] = useState<Update | null>(null);
  const { modules, missingFor } = useModules();

  const settingsRef = useRef<Settings | null>(null);
  settingsRef.current = settings;
  const tRef = useRef(t);
  tRef.current = t;
  const runningRef = useRef(running);
  runningRef.current = running;
  /** Batches whose events still matter ("Tout effacer" forgets the older ones). */
  const liveBatches = useRef(new Set<string>());
  const combineBatches = useRef(new Set<string>());
  /** Bumped by "Tout effacer": results of anything started before are dropped. */
  const generation = useRef(0);

  const showError = useCallback((e: unknown) => setToast({ kind: "error", text: errorText(tRef.current, e) }), []);

  // ---- settings: Rust is the source of truth ----
  useEffect(() => {
    api
      .getSettings()
      .then((s) => {
        setSettings(s);
        setLang(s.language);
      })
      .catch((e) => {
        setSettings(DEFAULT_SETTINGS);
        showError(e);
      });
  }, [setLang, showError]);

  // ---- new version? (a few seconds after start, never blocking) ----
  const checkUpdate = useCallback(async () => {
    try {
      const found = await api.checkUpdate();
      setUpdate(found);
      api.logUi("info", `update check: ${found ? `${found.version} available` : "up to date"}`).catch(() => {});
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
      showError(e);
      // Show what was really saved (the save may have worked and only the Explorer menu failed).
      api
        .getSettings()
        .then((s) => {
          setSettings(s);
          setLang(s.language);
        })
        .catch(() => {});
    }
  };

  // ---- toasts disappear by themselves (errors stay longer: they may need reading) ----
  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(null), toast.kind === "error" ? 12000 : 6000);
    return () => clearTimeout(id);
  }, [toast]);

  // ---- per-file progress, applied in small batches (hundreds of files → few renders) ----
  useEffect(() => {
    let pending: JobEvent[] = [];
    let timer: ReturnType<typeof setTimeout> | undefined;
    const flush = () => {
      timer = undefined;
      const events = pending.filter((e) => liveBatches.current.has(e.batchId));
      pending = [];
      const combined = events.filter((e) => combineBatches.current.has(e.batchId)).pop();
      const perFile = events.filter((e) => !combineBatches.current.has(e.batchId));
      if (combined) setCombineJob(combined);
      if (perFile.length)
        setJobs((prev) => {
          const next = { ...prev };
          for (const e of perFile) next[e.input] = e;
          return next;
        });
    };
    const off = listen<JobEvent>("job-event", (e) => {
      pending.push(e.payload);
      timer ??= setTimeout(flush, 50);
    });
    return () => {
      off.then((fn) => fn());
      if (timer) clearTimeout(timer);
    };
  }, []);

  const addPaths = useCallback(
    async (paths: string[]) => {
      const gen = generation.current;
      try {
        const infos = await api.inspectFiles(paths);
        if (gen !== generation.current) return []; // "Tout effacer" meanwhile
        setFiles((prev) => {
          const known = new Set(prev.map((f) => f.path));
          return [...prev, ...infos.filter((f) => !known.has(f.path))];
        });
        return infos;
      } catch (e) {
        showError(e);
        return [];
      }
    },
    [showError],
  );

  const { hovering } = useFileDrop(addPaths);

  const pickFiles = useCallback(async () => {
    const picked = await open({ multiple: true, directory: false }).catch(() => null);
    if (picked?.length) addPaths(picked);
  }, [addPaths]);
  const pickFolder = useCallback(async () => {
    const picked = await open({ multiple: true, directory: true }).catch(() => null);
    if (picked?.length) addPaths(picked);
  }, [addPaths]);

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
          }).catch(() => null);
          if (!outputPath) return;
        } else {
          const dir = await open({ directory: true }).catch(() => null);
          if (typeof dir !== "string") return;
          outputDir = dir;
        }
      }

      const batchId = crypto.randomUUID();
      const gen = generation.current;
      liveBatches.current.add(batchId);
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
        if (gen !== generation.current) return; // the list was cleared meanwhile
        // The returned results are the truth, even if a progress event got lost on the way.
        if (combine) setCombineJob(results[0]);
        else
          setJobs((prev) => {
            const next = { ...prev };
            for (const r of results) next[r.input] = r;
            return next;
          });

        // FFmpeg missing although we thought it was there (module list not loaded yet,
        // removed meanwhile): offer the download instead of a bare error.
        const missingCode = results.find((r) => r.errorCode?.startsWith("moduleMissing:"))?.errorCode;
        if (missingCode && !moduleJustInstalled) {
          setPendingRun({ moduleId: missingCode.slice("moduleMissing:".length), op, targets: compatible, options, combine });
          return;
        }
        const count = (s: JobEvent["status"]) => results.filter((r) => r.status === s).length;
        const [done, failed, cancelled] = [count("done"), count("failed"), count("cancelled")];
        if (results.length > 1 && !combine)
          setToast({
            kind: failed ? "error" : "ok",
            text: cancelled ? t("batch.summaryCancelled", { done, failed, cancelled }) : t("batch.summary", { done, failed }),
          });
      } catch (e) {
        showError(e);
      } finally {
        setRunning((r) => r.filter((id) => id !== batchId));
        combineBatches.current.delete(batchId);
        // Late events of a finished batch are harmless but useless.
        setTimeout(() => liveBatches.current.delete(batchId), 1000);
      }
    },
    [t, missingFor, showError],
  );
  // Stable entry point for memoized children and long-lived listeners.
  const runOpRef = useRef(runOp);
  runOpRef.current = runOp;

  // ---- Explorer context menu: files (and maybe an operation) passed at launch ----
  useEffect(() => {
    const handle = async (req: LaunchRequest | null) => {
      if (!req) return;
      const infos = await addPaths(req.files);
      if (!req.operation) return;
      const op = infos.flatMap((f) => f.operations).find((o) => o.id === req.operation);
      if (op) runOpRef.current(op, infos, rememberedOptions(op.id, op.options));
    };
    api.takeLaunchRequest().then(handle).catch(() => {});
    const off = listen<LaunchRequest>("launch-request", (e) => handle(e.payload));
    return () => {
      off.then((fn) => fn());
    };
  }, [addPaths]);

  // ---- closing the window while converting stops the conversions: ask first ----
  useEffect(() => {
    const off = getCurrentWindow().onCloseRequested(async (event) => {
      if (runningRef.current.length === 0) return;
      const sure = await ask(tRef.current("confirm.closeRunning"), { title: "Convix", kind: "warning" }).catch(() => true);
      if (!sure) event.preventDefault();
    });
    return () => {
      off.then((fn) => fn()).catch(() => {});
    };
  }, []);

  // ---- keyboard: Ctrl+O opens files (Ctrl+Shift+O a folder), Ctrl+, the settings ----
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.altKey || e.metaKey) return;
      if (e.key.toLowerCase() === "o") {
        e.preventDefault();
        if (e.shiftKey) pickFolder();
        else pickFiles();
      } else if (e.key === ",") {
        e.preventDefault();
        setShowSettings(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [pickFiles, pickFolder]);

  const removeFile = useCallback((path: string) => {
    releaseThumbnails([path]);
    setFiles((prev) => prev.filter((f) => f.path !== path));
    setJobs(({ [path]: _, ...rest }) => rest);
    setSelection((s) => (s?.scope === path ? null : s));
  }, []);
  const clearDone = () => {
    const done = new Set(Object.values(jobs).filter((j) => j.status === "done").map((j) => j.input));
    releaseThumbnails(done);
    setFiles((prev) => prev.filter((f) => !done.has(f.path)));
    setJobs((prev) => Object.fromEntries(Object.entries(prev).filter(([path]) => !done.has(path))));
  };
  const clearAll = () => {
    generation.current++;
    for (const id of running) api.cancelBatch(id).catch(() => {});
    liveBatches.current.clear();
    combineBatches.current.clear();
    releaseThumbnails(files.map((f) => f.path));
    setFiles([]);
    setJobs({});
    setCombineJob(undefined);
    setSelection(null);
  };
  const hasDone = useMemo(() => {
    const present = new Set(files.map((f) => f.path));
    return Object.values(jobs).some((j) => j.status === "done" && present.has(j.input));
  }, [files, jobs]);

  const onCardSelect = useCallback((f: FileInfo, op: OperationDescriptor | null) => setSelection(op ? { scope: f.path, op } : null), []);
  const onCardRun = useCallback((f: FileInfo, op: OperationDescriptor, values: OptionValues) => runOpRef.current(op, [f], values), []);
  const onCardRemove = useCallback((f: FileInfo) => removeFile(f.path), [removeFile]);
  const pendingModule = pendingRun ? modules.find((m) => m.id === pendingRun.moduleId) : undefined;

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
              onClick={() => running.forEach((id) => api.cancelBatch(id).catch(() => {}))}
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
            <button onClick={clearAll} className="rounded-lg px-3 py-1.5 text-sm text-muted hover:bg-surface-2">
              {t("files.clear")}
            </button>
          )}
          <button
            onClick={() => setShowSettings(true)}
            className="rounded-lg border border-border px-3 py-1.5 text-sm hover:bg-surface-2"
            title={`${t("app.settings")} (Ctrl+,)`}
          >
            <span aria-hidden>⚙</span> {t("app.settings")}
          </button>
        </div>
      </header>

      {update && <UpdateBanner update={update} busy={running.length > 0} onDismiss={() => setUpdate(null)} />}

      <DropZone hovering={hovering} compact={files.length > 0} onPickFiles={pickFiles} onPickFolder={pickFolder} />

      {files.length > 0 && (
        <section className="flex-1 space-y-3 overflow-y-auto pr-1" aria-label={t("files.count", { count: files.length })}>
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
              onSelect={onCardSelect}
              onRun={onCardRun}
              onRemove={onCardRemove}
            />
          ))}
        </section>
      )}

      {showSettings && settings && (
        <SettingsPanel settings={settings} onChange={changeSettings} onClose={() => setShowSettings(false)} onCheckUpdate={checkUpdate} />
      )}

      {pendingRun && pendingModule && (
        <ModulePrompt
          module={pendingModule}
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
          role={toast.kind === "error" ? "alert" : "status"}
          className={`selectable fixed bottom-4 left-1/2 z-40 flex max-w-[90%] -translate-x-1/2 items-start gap-3 rounded-lg px-4 py-2 text-sm shadow-lg ${
            toast.kind === "ok" ? "bg-success text-black" : "bg-danger text-on-danger"
          }`}
        >
          <span>{toast.text}</span>
          <button onClick={() => setToast(null)} aria-label={t("toast.close")} title={t("toast.close")} className="-mr-1 opacity-70 hover:opacity-100">
            ✕
          </button>
        </div>
      )}
    </div>
  );
}
