// Mirrors of the Rust types sent over IPC (crates/convertio-core, src-tauri/src).
// TODO: generate automatically (ts-rs / specta) once the API stabilizes.

export type Format =
  | "png" | "jpeg" | "webp" | "avif" | "gif" | "bmp" | "tiff" | "ico" | "heic" | "svg" | "tga"
  | "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" | "opus"
  | "mp4" | "mov" | "mkv" | "webm" | "avi"
  | "pdf";

export type MediaKind = "image" | "audio" | "video" | "document";

export type OperationCategory = "convert" | "compress" | "transform" | "extract" | "combine";

export interface Choice {
  value: string;
  label: string;
  literal: boolean;
}

export type OptionSpec =
  | { type: "slider"; id: string; labelKey: string; min: number; max: number; step: number; default: number; unit: string | null }
  | { type: "toggle"; id: string; labelKey: string; default: boolean }
  | { type: "select"; id: string; labelKey: string; choices: Choice[]; default: string }
  | { type: "number"; id: string; labelKey: string; min: number | null; max: number | null; default: number | null; unit: string | null };

export type OptionValues = Record<string, string | number | boolean | null>;

export interface OperationDescriptor {
  id: string;
  converter: string;
  category: OperationCategory;
  output: Format | null;
  outputSuffix: string | null;
  labelKey: string;
  multiInput: boolean;
  requires: string[];
  options: OptionSpec[];
}

export interface FileInfo {
  path: string;
  name: string;
  size: number;
  format: Format | null;
  formatLabel: string | null;
  kind: MediaKind | null;
  operations: OperationDescriptor[];
  error: string | null;
}

export interface LaunchRequest {
  operation: string | null;
  files: string[];
}

export type OutputPolicy =
  | { mode: "nextToSource" }
  | { mode: "fixedFolder"; path: string }
  | { mode: "askEachTime" };

export interface Settings {
  output: OutputPolicy;
  language: "fr" | "en";
  theme: "system" | "dark" | "light";
  shellIntegration: boolean;
}

export type JobStatus = "running" | "done" | "failed" | "cancelled";

export interface JobEvent {
  batchId: string;
  input: string;
  status: JobStatus;
  outputs: string[];
  inputSize: number;
  outputSize: number;
  error: string | null;
}

export type ModuleStatus =
  | { state: "notInstalled" }
  | { state: "installed"; version: string; path: string }
  | { state: "updateAvailable"; installed: string; latest: string; path: string };
