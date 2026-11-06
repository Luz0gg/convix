// Single entry point to the Rust side. Components never call `invoke` directly.
import { invoke } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import type { FileInfo, JobEvent, LaunchRequest, ModuleInfo, OptionValues, Settings } from "./types";

export const api = {
  appInfo: () => invoke<{ version: string; logPath: string | null }>("app_info"),
  logUi: (level: "error" | "info", message: string) => invoke<void>("log_ui", { level, message }),
  /** Signed update from GitHub Releases, or null when up to date. */
  checkUpdate: (): Promise<Update | null> => check(),
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
  thumbnail: (path: string) => invoke<ArrayBuffer>("thumbnail", { path }),
  rememberOptions: (operationId: string, options: OptionValues) => invoke<void>("remember_options", { operationId, options }),
  moduleList: () => invoke<ModuleInfo[]>("module_list"),
  /** Progress arrives as `module-progress` events. */
  installModule: (id: string) => invoke<ModuleInfo[]>("install_module", { id }),
  cancelModuleInstall: (id: string) => invoke<void>("cancel_batch", { batchId: `module:${id}` }),
  removeModule: (id: string) => invoke<ModuleInfo[]>("remove_module", { id }),
};

/** Canonical extension per format (keep in sync with `Format::extension` in Rust). */
export const ext = (format: string) => (format === "jpeg" ? "jpg" : format);

export const FORMAT_LABELS: Record<string, string> = {
  png: "PNG", jpeg: "JPEG", webp: "WebP", avif: "AVIF", gif: "GIF", bmp: "BMP", tiff: "TIFF", ico: "ICO",
  tga: "TGA", svg: "SVG", heic: "HEIC",
  mp3: "MP3", wav: "WAV", flac: "FLAC", aac: "AAC", m4a: "M4A", ogg: "OGG", opus: "Opus", aiff: "AIFF",
  mp4: "MP4", mov: "MOV", mkv: "MKV", webm: "WebM", avi: "AVI", pdf: "PDF",
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
export function rememberedOptions(opId: string, specs: { id: string; default?: unknown }[]): OptionValues {
  const defaults = Object.fromEntries(specs.map((s) => [s.id, s.default ?? null])) as OptionValues;
  try {
    const saved = JSON.parse(localStorage.getItem(`convix.opts.${opId}`) ?? "null");
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
    localStorage.setItem(`convix.opts.${opId}`, JSON.stringify(values));
  } catch {
    /* ignore */
  }
  // Also on the Rust side: right-click conversions run without the window.
  api.rememberOptions(opId, values).catch(() => {});
}

type Translate = (key: string, params?: Record<string, string | number>) => string;

/** Errors from Rust that only mean "the user stopped it": nothing to show. */
export const isCancelled = (e: unknown) => String(e) === "cancelled";

/**
 * Human text for an error coming from Rust. Known codes (`busy`, `io.locked`,
 * `moduleMissing:ffmpeg`…) are translated; anything else is shown as sent.
 */
export function errorText(t: Translate, error: unknown, code?: string | null): string {
  const raw = String(error ?? "");
  const c = code ?? raw;
  if (c.startsWith("moduleMissing:")) return t("error.moduleMissing", { module: c.slice("moduleMissing:".length) });
  if (c === "invalidOption") return t("error.invalidOption", { detail: raw });
  if (["busy", "cancelled", "unsupportedFormat", "unknownOperation", "io.notFound", "io.denied", "io.locked", "io.diskFull"].includes(c))
    return t(`error.${c}`);
  return raw;
}
