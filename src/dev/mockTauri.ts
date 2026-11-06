// DEV ONLY — lets the UI run in a plain browser (`pnpm dev`, http://localhost:1420) by faking
// the Rust side. Loaded by main.tsx only when Tauri is absent; never part of the real app.
// In the browser console: `__mockDrop()` simulates dropping files.
import type { FileInfo, OperationDescriptor, Settings } from "../lib/types";

type Cb = (payload: unknown) => void;
const callbacks = new Map<number, Cb>();
const listeners = new Map<string, number[]>();
let nextId = 1;

const emit = (event: string, payload: unknown) =>
  (listeners.get(event) ?? []).forEach((h) => callbacks.get(h)?.({ event, id: 0, payload }));

const convert = (to: string, options: OperationDescriptor["options"] = []): OperationDescriptor => ({
  id: `image.convert.${to === "jpeg" ? "jpg" : to}`, converter: "image", category: "convert",
  output: to as OperationDescriptor["output"], outputSuffix: null, labelKey: "op.convert", multiInput: false, requires: [], options,
});
const quality = (d: number) => ({ type: "slider" as const, id: "quality", labelKey: "opt.quality", min: 1, max: 100, step: 1, default: d, unit: "%" });

function opsFor(format: string): OperationDescriptor[] {
  const outs = ["png", "jpeg", "webp", "avif", "ico", "gif", "tiff", "bmp", "tga"].filter((f) => f !== format);
  const ops = outs.map((f) =>
    convert(f, f === "jpeg" ? [quality(90)] : f === "webp" ? [{ type: "toggle", id: "lossless", labelKey: "opt.lossless", default: false }, quality(90)] : f === "avif" ? [quality(75)] : []),
  );
  ops.push({ id: "image.compress.smaller", converter: "image", category: "compress", output: format as never, outputSuffix: "-min", labelKey: "op.compressSmaller", multiInput: false, requires: [], options: [quality(75)] });
  ops.push({
    id: "image.resize", converter: "image", category: "transform", output: format as never, outputSuffix: "-resized", labelKey: "op.resize", multiInput: false, requires: [],
    options: [
      { type: "number", id: "width", labelKey: "opt.width", min: 1, max: null, default: null, unit: "px" },
      { type: "number", id: "height", labelKey: "opt.height", min: 1, max: null, default: null, unit: "px" },
      { type: "toggle", id: "keepRatio", labelKey: "opt.keepRatio", default: true },
    ],
  });
  return ops;
}

const AUDIO = ["mp3", "wav", "flac", "m4a", "aac", "ogg", "opus", "aiff"];
const VIDEO = ["mp4", "mov", "mkv", "webm", "avi"];
const audioOptions = (to: string): OperationDescriptor["options"] => [
  ["wav", "flac", "aiff"].includes(to)
    ? { type: "select", id: "bitDepth", labelKey: "opt.bitDepth", choices: [{ value: "source", label: "opt.keepSource", literal: false }, { value: "16", label: "16 bits", literal: true }, { value: "24", label: "24 bits", literal: true }], default: "source" }
    : { type: "select", id: "bitrate", labelKey: "opt.bitrate", choices: ["320", "256", "192", "128", "96"].map((v) => ({ value: v, label: `${v} kbps`, literal: true })), default: "192" },
  { type: "select", id: "loudness", labelKey: "opt.loudness", choices: ["off", "streaming", "podcast", "broadcast"].map((v) => ({ value: v, label: `opt.loudness.${v}`, literal: false })), default: "off" },
];
function audioOpsFor(format: string): OperationDescriptor[] {
  const video = VIDEO.includes(format);
  const outs = video ? ["mp3", "wav", "flac", "m4a"] : AUDIO.filter((f) => f !== format);
  return outs.map((to) => ({
    id: `audio.${video ? "extract" : "convert"}.${to}`, converter: "audio", category: video ? "extract" : "convert",
    output: to as OperationDescriptor["output"], outputSuffix: null, labelKey: video ? "op.extractAudio" : "op.convert",
    multiInput: false, requires: ["ffmpeg"], options: audioOptions(to),
  }));
}
let ffmpegInstalled = false;
const moduleList = () => [{
  id: "ffmpeg", name: "FFmpeg", version: "9.0.2", license: "GPL-3.0 (build gyan.dev)", homepage: "https://www.gyan.dev/ffmpeg/builds/", downloadSize: 35_430_500,
  status: ffmpegInstalled ? { state: "installed", version: "9.0.2", path: "C:\\ffmpeg.exe" } : { state: "notInstalled" },
}];

let settings: Settings ={ output: { mode: "nextToSource" }, language: "fr", theme: "system", shellIntegration: true, quietContextMenu: true };

/** A colored gradient as a fake thumbnail. */
async function fakeThumbnail(path: string): Promise<ArrayBuffer> {
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d")!;
  const hue = [...path].reduce((h, ch) => h + ch.charCodeAt(0), 0) % 360;
  const grad = g.createLinearGradient(0, 0, 64, 64);
  grad.addColorStop(0, `hsl(${hue} 70% 55%)`);
  grad.addColorStop(1, `hsl(${(hue + 60) % 360} 70% 35%)`);
  g.fillStyle = grad;
  g.fillRect(0, 0, 64, 64);
  const blob = await new Promise<Blob>((r) => c.toBlob((b) => r(b!), "image/png"));
  return blob.arrayBuffer();
}

async function invoke(cmd: string, args: Record<string, any> = {}): Promise<unknown> {
  switch (cmd) {
    case "plugin:event|listen":
      listeners.set(args.event, [...(listeners.get(args.event) ?? []), args.handler]);
      return args.handler;
    case "plugin:event|unlisten":
      return null;
    case "app_info":
      return { version: "0.1.0-dev" };
    case "take_launch_request":
      return null;
    case "get_settings":
      return settings;
    case "update_settings":
      return (settings = args.settings);
    case "thumbnail":
      return fakeThumbnail(args.path);
    case "remember_options":
      return null;
    case "inspect_files":
      return (args.paths as string[]).map((path): FileInfo => {
        const name = path.split(/[\\/]/).pop()!;
        const e = name.split(".").pop()!.toLowerCase();
        const format = (e === "jpg" ? "jpeg" : e) as FileInfo["format"];
        const kind = AUDIO.includes(e) ? "audio" : VIDEO.includes(e) ? "video" : "image";
        const operations = kind === "image" ? opsFor(format!) : audioOpsFor(e);
        return { path, name, size: 200_000 + name.length * 91_000, format, formatLabel: format!.toUpperCase(), kind, operations, error: null };
      });
    case "module_list":
      return moduleList();
    case "remove_module":
      ffmpegInstalled = false;
      return moduleList();
    case "install_module": {
      const total = 35_430_500;
      for (let done = 0; done <= total; done += total / 25) {
        emit("module-progress", { id: args.id, phase: "downloading", done: Math.round(done), total });
        await new Promise((r) => setTimeout(r, 80));
      }
      emit("module-progress", { id: args.id, phase: "extracting", done: 0, total: 3 });
      await new Promise((r) => setTimeout(r, 600));
      ffmpegInstalled = true;
      return moduleList();
    }
    case "run_batch": {
      const events = (args.inputs as string[]).map((input, i) => {
        const base = { batchId: args.batchId, input, outputs: [], inputSize: 900_000, outputSize: 0, error: null, progress: null, errorCode: null };
        emit("job-event", { ...base, status: "running" });
        if (String(args.operationId).startsWith("audio.")) {
          for (let p = 1; p < 10; p++) setTimeout(() => emit("job-event", { ...base, status: "running", progress: p / 10 }), p * 70 + i * 300);
        }
        const ok = i % 4 !== 3;
        const final = ok
          ? { ...base, status: "done", outputs: [input.replace(/\.[^.]+$/, "-min.png")], outputSize: 210_000 + i * 50_000 }
          : { ...base, status: "failed", error: "format non supporté : exemple d'erreur" };
        setTimeout(() => emit("job-event", final), 700 + i * 300);
        return final;
      });
      await new Promise((r) => setTimeout(r, 800 + events.length * 300));
      return events;
    }
    default:
      console.info("[mockTauri] unhandled", cmd, args);
      return null;
  }
}

(window as any).__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
  transformCallback: (cb: Cb) => {
    const id = nextId++;
    callbacks.set(id, cb);
    return id;
  },
  unregisterCallback: (id: number) => callbacks.delete(id),
  invoke,
  convertFileSrc: (p: string) => p,
};

(window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };

(window as any).__mockDrop = (paths = ["C:\\Montage\\vignette.webp", "C:\\Montage\\interview.wav", "C:\\Montage\\rush_A001.mov"]) =>
  emit("tauri://drag-drop", { paths, position: { x: 0, y: 0 } });
