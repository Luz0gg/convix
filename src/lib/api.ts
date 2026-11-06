// Single entry point to the Rust side. Components never call `invoke` directly.
import { invoke } from "@tauri-apps/api/core";
import type { FileInfo, LaunchRequest, ModuleStatus } from "./types";

export const api = {
  appInfo: () => invoke<{ version: string }>("app_info"),
  takeLaunchRequest: () => invoke<LaunchRequest | null>("take_launch_request"),
  inspectFiles: (paths: string[]) => invoke<FileInfo[]>("inspect_files", { paths }),
  runOperation: (args: {
    operationId: string;
    inputs: string[];
    options: Record<string, unknown>;
    outputExt: string;
  }) => invoke<string[]>("run_operation", args),
  moduleStatus: (id: string) => invoke<ModuleStatus>("module_status", { id }),
  setShellIntegration: (enabled: boolean) => invoke<void>("set_shell_integration", { enabled }),
};

/** Canonical extension per format (keep in sync with `Format::extension` in Rust). */
export const extensionOf: Record<string, string> = {
  jpeg: "jpg",
};
export const ext = (format: string) => extensionOf[format] ?? format;

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  const units = ["Ko", "Mo", "Go"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}
