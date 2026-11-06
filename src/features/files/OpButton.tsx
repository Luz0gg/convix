import { useI18n } from "../../i18n";
import { opLabel } from "../../lib/ops";
import type { OperationDescriptor } from "../../lib/types";
import { useModules } from "../modules/modules";

export function OpButton({ op, active, count, onClick }: { op: OperationDescriptor; active: boolean; count?: number; onClick: () => void }) {
  const { t } = useI18n();
  const missing = useModules().missingFor(op);
  return (
    <button
      onClick={onClick}
      aria-pressed={active}
      title={missing.length ? t("module.required", { module: missing.map((m) => m.name).join(", ") }) : undefined}
      className={`rounded-lg border px-2.5 py-1 text-sm transition-colors ${
        active ? "border-accent bg-accent-soft text-accent" : "border-border bg-surface-2 hover:border-accent hover:text-accent"
      }`}
    >
      {opLabel(t, op)}
      {op.options.length > 0 && <span className="ml-1 text-xs text-muted">⚙</span>}
      {count !== undefined && <span className="ml-1.5 rounded bg-surface px-1 text-xs text-muted">{count}</span>}
      {missing.length > 0 && <span className="ml-1 text-xs text-muted">⬇</span>}
    </button>
  );
}
