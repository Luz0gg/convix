import { useI18n } from "../../i18n";

export function DropZone({ hovering, compact }: { hovering: boolean; compact: boolean }) {
  const { t } = useI18n();
  return (
    <div
      className={[
        "flex flex-col items-center justify-center rounded-2xl border-2 border-dashed text-center transition-colors",
        compact ? "py-6" : "flex-1 py-16",
        hovering ? "border-accent bg-accent-soft" : "border-border bg-surface",
      ].join(" ")}
    >
      <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
        className={hovering ? "text-accent" : "text-muted"} aria-hidden>
        <path d="M12 16V4m0 0-4 4m4-4 4 4M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
      <p className="mt-3 text-lg font-semibold">{hovering ? t("drop.release") : t("drop.title")}</p>
      {!compact && <p className="mt-1 max-w-sm text-sm text-muted">{t("drop.subtitle")}</p>}
    </div>
  );
}
