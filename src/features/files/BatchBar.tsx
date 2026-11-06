// "For all files": operations shared by several dropped files, applied in one click.
import { useI18n } from "../../i18n";
import { sharedOperations } from "../../lib/ops";
import type { FileInfo, OperationDescriptor, OptionValues } from "../../lib/types";
import { OptionsPanel } from "../options/OptionsPanel";
import { OpGroups } from "./OpGroups";

export function BatchBar({
  files,
  selectedOp,
  onSelect,
  onRun,
}: {
  files: FileInfo[];
  selectedOp: OperationDescriptor | null;
  onSelect: (op: OperationDescriptor | null) => void;
  onRun: (op: OperationDescriptor, values: OptionValues) => void;
}) {
  const { t } = useI18n();
  const shared = sharedOperations(files);
  if (shared.length === 0) return null;
  const counts = new Map(shared.map((s) => [s.op.id, s.count]));
  const pick = (op: OperationDescriptor) => (op.options.length ? onSelect(selectedOp?.id === op.id ? null : op) : onRun(op, {}));

  return (
    <section className="rounded-xl border border-accent/30 bg-surface p-4">
      <h2 className="mb-3 text-sm font-semibold">
        {t("batch.title")} <span className="font-normal text-muted">· {t("files.count", { count: files.length })}</span>
      </h2>
      <OpGroups ops={shared.map((s) => s.op)} counts={counts} selectedId={selectedOp?.id ?? null} onPick={pick} />
      {selectedOp && (
        <OptionsPanel op={selectedOp} count={counts.get(selectedOp.id) ?? 0} onRun={(v) => onRun(selectedOp, v)} onClose={() => onSelect(null)} />
      )}
    </section>
  );
}
