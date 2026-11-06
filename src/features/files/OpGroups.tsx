// Operation buttons grouped by category. In "Convert", the main formats come first and
// the rest fold behind a "+N" button to keep cards compact.
import { useState } from "react";
import { useI18n } from "../../i18n";
import { groupByCategory } from "../../lib/ops";
import type { OperationDescriptor } from "../../lib/types";
import { OpButton } from "./OpButton";

const PRIMARY_OUTPUTS = new Set(["png", "jpeg", "webp", "avif", "mp3", "wav", "pdf"]);

export function OpGroups({
  ops,
  selectedId,
  counts,
  onPick,
}: {
  ops: OperationDescriptor[];
  selectedId: string | null;
  counts?: Map<string, number>;
  onPick: (op: OperationDescriptor) => void;
}) {
  const { t } = useI18n();
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="space-y-2">
      {groupByCategory(ops).map(([category, list]) => {
        const foldable = category === "convert" && list.length > 5;
        const primary = foldable ? list.filter((o) => PRIMARY_OUTPUTS.has(o.output ?? "")) : list;
        const extra = foldable ? list.filter((o) => !PRIMARY_OUTPUTS.has(o.output ?? "")) : [];
        const visible = expanded || extra.some((o) => o.id === selectedId) ? list : primary;
        return (
          <div key={category} className="flex flex-wrap items-center gap-1.5">
            <span className="w-24 shrink-0 text-xs uppercase tracking-wide text-muted">{t(`category.${category}`)}</span>
            {visible.map((op) => (
              <OpButton key={op.id} op={op} active={selectedId === op.id} count={counts?.get(op.id)} onClick={() => onPick(op)} />
            ))}
            {extra.length > 0 && visible.length < list.length && (
              <button onClick={() => setExpanded(true)} className="rounded-lg px-2 py-1 text-sm text-muted hover:text-accent">
                +{extra.length}
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
