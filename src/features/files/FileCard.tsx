import { memo } from "react";
import { useI18n } from "../../i18n";
import { errorText, formatSize } from "../../lib/api";
import type { FileInfo, JobEvent, OperationDescriptor, OptionValues } from "../../lib/types";
import { OptionsPanel } from "../options/OptionsPanel";
import { JobStatusLine } from "./JobStatusLine";
import { OpGroups } from "./OpGroups";
import { Thumbnail } from "./Thumbnail";

/**
 * One dropped file. Memoized: with hundreds of files, a progress event only re-renders
 * the card it is about. Callbacks receive the file, so the parent can pass stable ones.
 */
export const FileCard = memo(function FileCard({
  file,
  job,
  selectedOp,
  onSelect,
  onRun,
  onRemove,
}: {
  file: FileInfo;
  job: JobEvent | undefined;
  selectedOp: OperationDescriptor | null;
  onSelect: (file: FileInfo, op: OperationDescriptor | null) => void;
  onRun: (file: FileInfo, op: OperationDescriptor, values: OptionValues) => void;
  onRemove: (file: FileInfo) => void;
}) {
  const { t } = useI18n();
  // "Merge PDFs" needs several files: it lives in the batch bar, not on a single card.
  const ops = file.operations.filter((o) => !(o.multiInput && o.output === file.format));
  const pick = (op: OperationDescriptor) =>
    op.options.length ? onSelect(file, selectedOp?.id === op.id ? null : op) : onRun(file, op, {});

  return (
    <article className="rounded-xl border border-border bg-surface p-4" aria-label={file.name}>
      <header className="mb-3 flex items-center gap-3">
        <Thumbnail file={file} />
        <div className="min-w-0 flex-1">
          <p className="selectable truncate font-medium" title={file.path}>
            {file.name}
          </p>
          <p className="text-xs text-muted">
            {formatSize(file.size)}
            {!file.format && ` · ${t("files.unknown")}`}
          </p>
        </div>
        <button
          onClick={() => onRemove(file)}
          className="rounded-md px-2 py-1 text-muted hover:bg-surface-2 hover:text-text"
          aria-label={t("files.remove", { name: file.name })}
          title={t("files.remove", { name: file.name })}
        >
          ✕
        </button>
      </header>

      {file.error ? (
        <p className="selectable text-sm text-danger">{errorText(t, file.error)}</p>
      ) : ops.length === 0 ? (
        <p className="text-sm text-muted">{t("files.noOperation")}</p>
      ) : (
        <OpGroups ops={ops} selectedId={selectedOp?.id ?? null} onPick={pick} />
      )}

      {selectedOp && (
        <OptionsPanel op={selectedOp} count={1} onRun={(v) => onRun(file, selectedOp, v)} onClose={() => onSelect(file, null)} />
      )}
      {job && <JobStatusLine job={job} />}
    </article>
  );
});
