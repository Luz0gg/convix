import { useI18n } from "../../i18n";
import { formatSize } from "../../lib/api";
import type { FileInfo, JobEvent, OperationDescriptor, OptionValues } from "../../lib/types";
import { OptionsPanel } from "../options/OptionsPanel";
import { JobStatusLine } from "./JobStatusLine";
import { OpGroups } from "./OpGroups";

export function FileCard({
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
  onSelect: (op: OperationDescriptor | null) => void;
  onRun: (op: OperationDescriptor, values: OptionValues) => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();
  const pick = (op: OperationDescriptor) => (op.options.length ? onSelect(selectedOp?.id === op.id ? null : op) : onRun(op, {}));

  return (
    <article className="rounded-xl border border-border bg-surface p-4">
      <header className="mb-3 flex items-center gap-3">
        <span className="rounded-md bg-surface-2 px-2 py-1 text-xs font-bold tracking-wide text-accent">{file.formatLabel ?? "?"}</span>
        <div className="min-w-0 flex-1">
          <p className="truncate font-medium" title={file.path}>{file.name}</p>
          <p className="text-xs text-muted">
            {formatSize(file.size)}
            {!file.format && ` · ${t("files.unknown")}`}
          </p>
        </div>
        <button onClick={onRemove} className="rounded-md px-2 py-1 text-muted hover:bg-surface-2 hover:text-text" aria-label="remove">✕</button>
      </header>

      {file.operations.length === 0 ? (
        <p className="text-sm text-muted">{t("files.noOperation")}</p>
      ) : (
        <OpGroups ops={file.operations} selectedId={selectedOp?.id ?? null} onPick={pick} />
      )}

      {selectedOp && <OptionsPanel op={selectedOp} count={1} onRun={(v) => onRun(selectedOp, v)} onClose={() => onSelect(null)} />}
      {job && <JobStatusLine job={job} />}
    </article>
  );
}
