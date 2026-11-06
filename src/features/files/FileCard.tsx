import { useI18n } from "../../i18n";
import { formatSize } from "../../lib/api";
import type { FileInfo, OperationCategory, OperationDescriptor } from "../../lib/types";

const CATEGORY_ORDER: OperationCategory[] = ["convert", "compress", "transform", "extract", "combine"];

const FORMAT_LABELS: Record<string, string> = {
  png: "PNG", jpeg: "JPEG", webp: "WebP", avif: "AVIF", gif: "GIF", bmp: "BMP", tiff: "TIFF", ico: "ICO",
  mp3: "MP3", wav: "WAV", flac: "FLAC", aac: "AAC", m4a: "M4A", ogg: "OGG", opus: "Opus", pdf: "PDF",
};

export function FileCard({
  file,
  onRun,
  onRemove,
}: {
  file: FileInfo;
  onRun: (file: FileInfo, op: OperationDescriptor) => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();

  const label = (op: OperationDescriptor) =>
    t(op.labelKey, { format: op.output ? FORMAT_LABELS[op.output] ?? op.output.toUpperCase() : "" });

  const groups = CATEGORY_ORDER.map((c) => [c, file.operations.filter((o) => o.category === c)] as const).filter(
    ([, ops]) => ops.length > 0,
  );

  return (
    <article className="rounded-xl border border-border bg-surface p-4">
      <header className="flex items-center gap-3">
        <span className="rounded-md bg-surface-2 px-2 py-1 text-xs font-bold tracking-wide text-accent">
          {file.formatLabel ?? "?"}
        </span>
        <div className="min-w-0 flex-1">
          <p className="truncate font-medium" title={file.path}>{file.name}</p>
          <p className="text-xs text-muted">
            {formatSize(file.size)}
            {!file.format && ` · ${t("files.unknown")}`}
          </p>
        </div>
        <button onClick={onRemove} className="rounded-md px-2 py-1 text-muted hover:bg-surface-2 hover:text-text" aria-label="remove">
          ✕
        </button>
      </header>

      {groups.length === 0 ? (
        <p className="mt-3 text-sm text-muted">{t("files.noOperation")}</p>
      ) : (
        <div className="mt-3 space-y-2">
          {groups.map(([category, ops]) => (
            <div key={category} className="flex flex-wrap items-center gap-1.5">
              <span className="w-24 shrink-0 text-xs uppercase tracking-wide text-muted">{t(`category.${category}`)}</span>
              {ops.map((op) => (
                <button
                  key={op.id}
                  onClick={() => onRun(file, op)}
                  title={op.requires.length ? t("module.required", { module: op.requires.join(", ") }) : undefined}
                  className="rounded-lg border border-border bg-surface-2 px-2.5 py-1 text-sm hover:border-accent hover:text-accent"
                >
                  {label(op)}
                  {op.requires.length > 0 && <span className="ml-1 text-xs text-muted">⬇</span>}
                </button>
              ))}
            </div>
          ))}
        </div>
      )}
    </article>
  );
}
