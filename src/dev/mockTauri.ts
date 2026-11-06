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

let settings: Settings = { output: { mode: "nextToSource" }, language: "fr", theme: "system", shellIntegration: true };

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
    case "inspect_files":
      return (args.paths as string[]).map((path): FileInfo => {
        const name = path.split(/[\\/]/).pop()!;
        const e = name.split(".").pop()!.toLowerCase();
        const format = (e === "jpg" ? "jpeg" : e) as FileInfo["format"];
        return { path, name, size: 200_000 + name.length * 91_000, format, formatLabel: format!.toUpperCase(), kind: "image", operations: opsFor(format!), error: null };
      });
    case "run_batch": {
      const events = (args.inputs as string[]).map((input, i) => {
        const base = { batchId: args.batchId, input, outputs: [], inputSize: 900_000, outputSize: 0, error: null };
        emit("job-event", { ...base, status: "running" });
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

(window as any).__mockDrop =(paths = ["C:\\Montage\\vignette.webp", "C:\\Montage\\logo.png", "C:\\Montage\\photo_iphone.jpg"]) =>
  emit("tauri://drag-drop", { paths, position: { x: 0, y: 0 } });
