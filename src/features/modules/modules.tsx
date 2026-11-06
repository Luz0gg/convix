// Installed external modules (FFmpeg…) and their downloads, shared by the whole UI.
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../../lib/api";
import type { ModuleInfo, ModuleProgress, OperationDescriptor } from "../../lib/types";

interface ModulesValue {
  modules: ModuleInfo[];
  /** Download in progress, by module id. */
  progress: Record<string, ModuleProgress>;
  /** The module list has been loaded (until then, `missingFor` can't know). */
  ready: boolean;
  /** Modules this operation needs that are not installed yet. */
  missingFor: (op: OperationDescriptor) => ModuleInfo[];
  install: (id: string) => Promise<void>;
  cancel: (id: string) => void;
  remove: (id: string) => Promise<void>;
}

const ModulesContext = createContext<ModulesValue | null>(null);

export function ModulesProvider({ children }: { children: ReactNode }) {
  const [modules, setModules] = useState<ModuleInfo[]>([]);
  const [progress, setProgress] = useState<Record<string, ModuleProgress>>({});
  const [ready, setReady] = useState(false);

  useEffect(() => {
    api
      .moduleList()
      .then(setModules)
      .catch(() => {})
      .finally(() => setReady(true));
    const off = listen<ModuleProgress>("module-progress", (e) => setProgress((p) => ({ ...p, [e.payload.id]: e.payload })));
    return () => {
      off.then((fn) => fn());
    };
  }, []);

  const missingFor = useCallback(
    (op: OperationDescriptor) => modules.filter((m) => op.requires.includes(m.id) && m.status.state === "notInstalled"),
    [modules],
  );

  const install = useCallback(async (id: string) => {
    setProgress((p) => ({ ...p, [id]: { id, phase: "downloading", done: 0, total: 0 } }));
    try {
      setModules(await api.installModule(id));
    } finally {
      setProgress(({ [id]: _, ...rest }) => rest);
    }
  }, []);

  const cancel = useCallback((id: string) => {
    api.cancelModuleInstall(id).catch(() => {});
  }, []);

  const remove = useCallback(async (id: string) => {
    setModules(await api.removeModule(id));
  }, []);

  // Memoized: every download progress event would otherwise re-render every action button.
  const value = useMemo(
    () => ({ modules, progress, ready, missingFor, install, cancel, remove }),
    [modules, progress, ready, missingFor, install, cancel, remove],
  );
  return <ModulesContext.Provider value={value}>{children}</ModulesContext.Provider>;
}

export function useModules(): ModulesValue {
  const value = useContext(ModulesContext);
  if (!value) throw new Error("useModules outside ModulesProvider");
  return value;
}
