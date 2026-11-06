// Several files → one result (merge PDFs, images → PDF): choose the order, then run.
import { useEffect, useState } from "react";
import { useI18n } from "../../i18n";
import { supports } from "../../lib/ops";
import type { FileInfo, OperationDescriptor, OptionValues } from "../../lib/types";
import { OptionsPanel } from "../options/OptionsPanel";
import { Thumbnail } from "./Thumbnail";

export function CombinePanel({
  op,
  files,
  onRun,
  onClose,
}: {
  op: OperationDescriptor;
  files: FileInfo[];
  onRun: (values: OptionValues, ordered: FileInfo[]) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [order, setOrder] = useState(() => files.filter((f) => supports(f, op.id)));
  // Files added or removed meanwhile: keep the order already chosen, new files go last.
  useEffect(
    () =>
      setOrder((current) => {
        const eligible = files.filter((f) => supports(f, op.id));
        const present = new Set(eligible.map((f) => f.path));
        const kept = current.filter((f) => present.has(f.path));
        const known = new Set(kept.map((f) => f.path));
        return [...kept, ...eligible.filter((f) => !known.has(f.path))];
      }),
    [files, op.id],
  );

  const move = (i: number, delta: number) =>
    setOrder((list) => {
      const j = i + delta;
      if (j < 0 || j >= list.length) return list;
      const next = [...list];
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });

  return (
    <div className="mt-3 space-y-2">
      <p className="text-xs text-muted">{t("combine.order")}</p>
      <ol className="max-h-64 space-y-1 overflow-y-auto">
        {order.map((f, i) => (
          <li key={f.path} className="flex items-center gap-2 rounded-lg bg-surface-2 px-2 py-1 text-sm">
            <span className="w-6 text-right text-xs tabular-nums text-muted">{i + 1}</span>
            <div className="scale-[0.6] -m-3">
              <Thumbnail file={f} />
            </div>
            <span className="min-w-0 flex-1 truncate" title={f.path}>{f.name}</span>
            <button onClick={() => move(i, -1)} disabled={i === 0} className="rounded px-1.5 text-muted hover:text-accent disabled:opacity-30" aria-label={t("combine.up", { name: f.name })} title={t("combine.up", { name: f.name })}>↑</button>
            <button onClick={() => move(i, 1)} disabled={i === order.length - 1} className="rounded px-1.5 text-muted hover:text-accent disabled:opacity-30" aria-label={t("combine.down", { name: f.name })} title={t("combine.down", { name: f.name })}>↓</button>
            <button onClick={() => setOrder((l) => l.filter((x) => x.path !== f.path))} className="rounded px-1.5 text-muted hover:text-danger" aria-label={t("combine.leaveOut", { name: f.name })} title={t("combine.leaveOut", { name: f.name })}>✕</button>
          </li>
        ))}
      </ol>
      <OptionsPanel
        op={op}
        count={order.length}
        onRun={(values) => onRun(values, order)}
        onClose={onClose}
        disabledReason={order.length < 2 ? t("combine.empty") : undefined}
      />
    </div>
  );
}
