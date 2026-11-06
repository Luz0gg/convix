// Single entry point to the Rust side. Components never call `invoke` directly.
import { invoke } from "@tauri-apps/api/core";
import type { FileInfo, JobEvent, LaunchRequest, ModuleStatus, OptionValues, Settings } from "./types";

export const api = {
  appInfo: () => invoke<{ version: string }>("app_info"),
  takeLaunchRequest: () => invoke<LaunchRequest | null>("take_launch_request"),
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings: Settings) => invoke<Settings>("update_settings", { settings }),
  inspectFiles: (paths: string[]) => invoke<FileInfo[]>("inspect_files", { paths }),
  runBatch: (args: {
    batchId: string;
    operationId: string;
    inputs: string[];
    options: OptionValues;
    outputDir?: string | null;
    outputPath?: string | null;
  }) => invoke<JobEvent[]>("run_batch", args),
  cancelBatch: (batchId: string) => invoke<void>("cancel_batch", { batchId }),
  moduleStatus: (id: string) => invoke<ModuleStatus>("module_status", { id }),
};

/** Canonical extension per format (keep in sync with `Format::extension` in Rust). */
export const ext = (format: string) => (format === "jpeg" ? "jpg" : format);

export const FORMAT_LABELS: Record<string, string> = {
  png: "PNG", jpeg: "JPEG", webp: "WebP", avif: "AVIF", gif: "GIF", bmp: "BMP", tiff: "TIFF", ico: "ICO",
  tga: "TGA", svg: "SVG", heic: "HEIC",
  mp3: "MP3", wav: "WAV", flac: "FLAC", aac: "AAC", m4a: "M4A", ogg: "OGG", opus: "Opus", pdf: "PDF",
};

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

export const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
export const dirName = (path: string) => path.replace(/[\\/][^\\/]*$/, "");

/** Default values of an operation's options, overridden by what the user used last time. */
export function rememberedOptions(opId: string, specs: { id: string; default: unknown }[]): OptionValues {
  const defaults = Object.fromEntries(specs.map((s) => [s.id, s.default ?? null])) as OptionValues;
  try {
    const saved = JSON.parse(localStorage.getItem(`convertio.opts.${opId}`) ?? "null");
    if (saved && typeof saved === "object") {
      for (const key of Object.keys(defaults)) if (key in saved) defaults[key] = saved[key];
    }
  } catch {
    /* storage unavailable */
  }
  return defaults;
}

export function rememberOptions(opId: string, values: OptionValues) {
  try {
    localStorage.setItem(`convertio.opts.${opId}`, JSON.stringify(values));
  } catch {
    /* ignore */
  }
}
