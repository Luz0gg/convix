import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "./i18n";
import { api, ext } from "./lib/api";
import type { FileInfo, LaunchRequest, OperationDescriptor } from "./lib/types";
import { DropZone } from "./features/drop/DropZone";
import { useFileDrop } from "./features/drop/useFileDrop";
import { FileCard } from "./features/files/FileCard";

type Toast = { kind: "ok" | "error"; text: string };

export default function App() {
  const { t, lang, setLang } = useI18n();
  const [files, setFiles] = useState<FileInfo[]>([]);
  const [toast, setToast] = useState<Toast | null>(null);

  const addPaths = useCallback(async (paths: string[]) => {
    const infos = await api.inspectFiles(paths);
    setFiles((prev) => {
      const known = new Set(prev.map((f) => f.path));
      return [...prev, ...infos.filter((f) => !known.has(f.path))];
    });
  }, []);

  const { hovering } = useFileDrop(addPaths);

  // Files passed by the Explorer context menu (at startup or to the already-open window).
  // TODO(Phase 1): when `operation` is set, run it directly instead of just listing the files.
  useEffect(() => {
    const handle = (req: LaunchRequest | null) => req && addPaths(req.files);
    api.takeLaunchRequest().then(handle);
    const off = listen<LaunchRequest>("launch-request", (e) => handle(e.payload));
    return () => {
      off.then((fn) => fn());
    };
  }, [addPaths]);

  const run = async (file: FileInfo, op: OperationDescriptor) => {
    const options = Object.fromEntries(op.options.map((o) => [o.id, o.default]));
    try {
      const out = await api.runOperation({
        operationId: op.id,
        inputs: [file.path],
        options,
        outputExt: ext(op.output ?? file.format ?? ""),
      });
      setToast({ kind: "ok", text: t("job.done", { file: out.join(", ") }) });
    } catch (e) {
      setToast({ kind: "error", text: t("job.failed", { error: String(e) }) });
    }
  };

  return (
    <div className="flex h-full flex-col gap-4 p-5">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">
            Convert<span className="text-accent">IO</span>
          </h1>
          <p className="text-sm text-muted">{t("app.tagline")}</p>
        </div>
        <div className="flex gap-2">
          {files.length > 0 && (
            <button onClick={() => setFiles([])} className="rounded-lg px-3 py-1.5 text-sm text-muted hover:bg-surface-2">
              {t("files.clear")}
            </button>
          )}
          <button
            onClick={() => setLang(lang === "fr" ? "en" : "fr")}
            className="rounded-lg border border-border px-3 py-1.5 text-sm hover:bg-surface-2"
          >
            {t("lang.switch")}
          </button>
        </div>
      </header>

      <DropZone hovering={hovering} compact={files.length > 0} />

      {files.length > 0 && (
        <section className="flex-1 space-y-3 overflow-y-auto pr-1">
          {files.map((f) => (
            <FileCard
              key={f.path}
              file={f}
              onRun={run}
              onRemove={() => setFiles((prev) => prev.filter((x) => x.path !== f.path))}
            />
          ))}
        </section>
      )}

      {toast && (
        <div
          onClick={() => setToast(null)}
          className={`fixed bottom-4 left-1/2 max-w-[90%] -translate-x-1/2 cursor-pointer rounded-lg px-4 py-2 text-sm shadow-lg ${
            toast.kind === "ok" ? "bg-success text-black" : "bg-danger text-white"
          }`}
        >
          {toast.text}
        </div>
      )}
    </div>
  );
}
