// Renders an operation's options from its OptionSpec list — no per-operation UI code.
import { useEffect, useState } from "react";
import { useI18n } from "../../i18n";
import { rememberOptions, rememberedOptions } from "../../lib/api";
import { opLabel } from "../../lib/ops";
import type { OperationDescriptor, OptionSpec, OptionValues } from "../../lib/types";

export function OptionsPanel({
  op,
  count,
  onRun,
  onClose,
  disabledReason,
}: {
  op: OperationDescriptor;
  count: number;
  onRun: (values: OptionValues) => void;
  onClose: () => void;
  /** When set, the run button is disabled and this explains why. */
  disabledReason?: string;
}) {
  const { t } = useI18n();
  const [values, setValues] = useState<OptionValues>(() => rememberedOptions(op.id, op.options));
  useEffect(() => setValues(rememberedOptions(op.id, op.options)), [op]);

  const set = (id: string, v: OptionValues[string]) => setValues((prev) => ({ ...prev, [id]: v }));
  const run = () => {
    // Page lists belong to one document: never carried over to the next one.
    const textIds = new Set(op.options.filter((s) => s.type === "text").map((s) => s.id));
    rememberOptions(op.id, Object.fromEntries(Object.entries(values).filter(([k]) => !textIds.has(k))));
    onRun(values);
  };

  return (
    <div className="mt-3 rounded-lg border border-accent/40 bg-surface-2 p-3">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-sm font-semibold text-accent">{opLabel(t, op)}</span>
        <button onClick={onClose} className="text-xs text-muted hover:text-text">{t("run.close")}</button>
      </div>
      <div className="space-y-3">
        {op.options.map((spec) => (
          <Field key={spec.id} spec={spec} value={values[spec.id]} onChange={(v) => set(spec.id, v)} />
        ))}
      </div>
      {disabledReason && <p className="mt-3 text-xs text-muted">{disabledReason}</p>}
      <button
        onClick={run}
        disabled={Boolean(disabledReason)}
        className="mt-3 w-full rounded-lg bg-accent px-3 py-2 text-sm font-semibold text-on-accent hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {count > 1 ? t("run.startCount", { count }) : t("run.start")}
      </button>
    </div>
  );
}

function Field({ spec, value, onChange }: { spec: OptionSpec; value: OptionValues[string]; onChange: (v: OptionValues[string]) => void }) {
  const { t } = useI18n();
  const label = <span className="text-xs text-muted">{t(spec.labelKey)}</span>;

  switch (spec.type) {
    case "slider":
      return (
        <label className="block">
          <div className="flex justify-between">
            {label}
            <span className="text-xs font-medium">{String(value)}{spec.unit ?? ""}</span>
          </div>
          <input
            type="range"
            min={spec.min}
            max={spec.max}
            step={spec.step}
            value={Number(value)}
            onChange={(e) => onChange(Number(e.target.value))}
            className="mt-1 w-full accent-[var(--color-accent)]"
          />
        </label>
      );
    case "toggle":
      return (
        <label className="flex cursor-pointer items-center justify-between">
          {label}
          <input type="checkbox" checked={Boolean(value)} onChange={(e) => onChange(e.target.checked)} className="h-4 w-4 accent-[var(--color-accent)]" />
        </label>
      );
    case "select":
      return (
        <div>
          {label}
          <div className="mt-1 flex flex-wrap gap-1">
            {spec.choices.map((c) => (
              <button
                key={c.value}
                onClick={() => onChange(c.value)}
                aria-pressed={value === c.value}
                className={`rounded-md border px-2 py-1 text-xs ${
                  value === c.value ? "border-accent bg-accent-soft text-accent" : "border-border hover:border-accent"
                }`}
              >
                {c.literal ? c.label : t(c.label)}
              </button>
            ))}
          </div>
        </div>
      );
    case "text":
      return (
        <label className="block">
          {label}
          <input
            type="text"
            placeholder={t(spec.placeholderKey)}
            value={value == null ? "" : String(value)}
            onChange={(e) => onChange(e.target.value)}
            className="mt-1 w-full rounded-md border border-border bg-surface px-2 py-1 text-sm focus:border-accent"
          />
        </label>
      );
    case "number":
      return (
        <label className="flex items-center justify-between gap-3">
          {label}
          <span className="flex items-center gap-1">
            <input
              type="number"
              min={spec.min ?? undefined}
              max={spec.max ?? undefined}
              placeholder={t("opt.auto")}
              value={value == null ? "" : String(value)}
              onChange={(e) => onChange(e.target.value === "" ? null : Number(e.target.value))}
              className="w-24 rounded-md border border-border bg-surface px-2 py-1 text-right text-sm focus:border-accent"
            />
            {spec.unit && <span className="text-xs text-muted">{spec.unit}</span>}
          </span>
        </label>
      );
  }
}
