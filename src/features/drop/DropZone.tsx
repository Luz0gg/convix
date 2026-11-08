import { useI18n } from "../../i18n";

export function DropZone({
  hovering,
  compact,
  onPickFiles,
  onPickFolder,
}: {
  hovering: boolean;
  compact: boolean;
  /** Keyboard / no-drag alternative to drag and drop. */
  onPickFiles: () => void;
  onPickFolder: () => void;
}) {
  const { t } = useI18n();
  return (
    <div
      className={[
        "flex flex-col items-center justify-center rounded-2xl border-2 border-dashed px-4 text-center transition-colors",
        compact ? "py-5" : "flex-1 py-16",
        hovering ? "border-accent bg-accent-soft" : "border-border bg-surface",
      ].join(" ")}
    >
      <svg
        width="40"
        height="40"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        className={hovering ? "text-accent" : "text-muted"}
        aria-hidden
      >
        <path d="M12 16V4m0 0-4 4m4-4 4 4M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
      <p className="mt-3 text-lg font-semibold">{hovering ? t("drop.release") : t("drop.title")}</p>
      {!compact && <p className="mt-1 max-w-sm text-sm text-muted">{t("drop.subtitle")}</p>}
      {!compact && !hovering && (
        <ul className="mt-4 flex flex-wrap justify-center gap-1.5" aria-label={t("drop.families")}>
          {["drop.family.image", "drop.family.audio", "drop.family.video", "drop.family.pdf"].map((key) => (
            <li key={key} className="rounded-full border border-border bg-surface-2 px-2.5 py-0.5 text-xs text-muted">
              {t(key)}
            </li>
          ))}
        </ul>
      )}
      {!hovering && (
        <div className="mt-3 flex flex-wrap items-center justify-center gap-2 text-sm">
          <button onClick={onPickFiles} className="rounded-lg border border-border bg-surface-2 px-3 py-1.5 hover:border-accent hover:text-accent">
            {t("drop.pick")}
          </button>
          <button onClick={onPickFolder} className="rounded-lg px-3 py-1.5 text-muted hover:bg-surface-2 hover:text-text">
            {t("drop.pickFolder")}
          </button>
          <span className="text-xs text-muted">{t("drop.hint")}</span>
        </div>
      )}
    </div>
  );
}
