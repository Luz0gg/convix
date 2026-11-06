import { open } from "@tauri-apps/plugin-dialog";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useState, type ReactNode } from "react";
import { useI18n } from "../../i18n";
import { api, errorText, formatSize, isCancelled } from "../../lib/api";
import type { Settings } from "../../lib/types";
import { ModuleProgressBar } from "../modules/ModuleProgressBar";
import { useModules } from "../modules/modules";

function Segmented<V extends string>({ value, options, onChange }: { value: V; options: [V, string][]; onChange: (v: V) => void }) {
  return (
    <div className="inline-flex rounded-lg border border-border p-0.5">
      {options.map(([v, label]) => (
        <button
          key={v}
          onClick={() => onChange(v)}
          className={`rounded-md px-3 py-1 text-sm ${value === v ? "bg-accent text-white" : "text-muted hover:text-text"}`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="space-y-2">
      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted">{title}</h3>
      {children}
    </section>
  );
}

/** External tools downloaded on demand (FFmpeg…): status, download, removal. */
function ModulesList() {
  const { t } = useI18n();
  const { modules, progress, install, cancel, remove } = useModules();
  const [error, setError] = useState<string | null>(null);
  const run = (action: () => Promise<void>) => {
    setError(null);
    action().catch((e) => {
      if (!isCancelled(e)) setError(errorText(t, e));
    });
  };

  return (
    <div className="space-y-3">
      {modules.map((m) => {
        const current = progress[m.id];
        const installed = m.status.state !== "notInstalled";
        const version = m.status.state === "installed" ? m.status.version : m.status.state === "updateAvailable" ? m.status.installed : null;
        return (
          <div key={m.id} className="space-y-2 rounded-lg border border-border p-3">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <p className="text-sm font-medium">
                  {m.name} {version && <span className="text-xs font-normal text-muted">v{version}</span>}
                </p>
                <p className="text-xs text-muted">{t(`module.${m.id}.why`)}</p>
              </div>
              {current ? (
                <button onClick={() => cancel(m.id)} className="shrink-0 rounded-md border border-danger px-2 py-1 text-xs text-danger">
                  {t("run.cancel")}
                </button>
              ) : m.status.state === "updateAvailable" ? (
                <button onClick={() => run(() => install(m.id))} className="shrink-0 rounded-md bg-accent px-2 py-1 text-xs text-white">
                  {t("module.update", { version: m.version })}
                </button>
              ) : installed ? (
                <button onClick={() => run(() => remove(m.id))} className="shrink-0 rounded-md border border-border px-2 py-1 text-xs hover:border-danger hover:text-danger">
                  {t("module.remove")}
                </button>
              ) : (
                <button onClick={() => run(() => install(m.id))} className="shrink-0 rounded-md bg-accent px-2 py-1 text-xs text-white">
                  {t("module.prompt.download", { size: formatSize(m.downloadSize) })}
                </button>
              )}
            </div>
            {current && <ModuleProgressBar progress={current} />}
            <p className="text-[11px] text-muted">{t("module.prompt.license", { license: m.license })}</p>
          </div>
        );
      })}
      {error && <p className="text-xs text-danger">{error}</p>}
    </div>
  );
}

type CheckResult = "found" | "upToDate" | "error";

export function SettingsPanel({
  settings,
  onChange,
  onClose,
  onCheckUpdate,
}: {
  settings: Settings;
  onChange: (s: Settings) => void;
  onClose: () => void;
  onCheckUpdate: () => Promise<CheckResult>;
}) {
  const { t } = useI18n();
  const [version, setVersion] = useState("");
  const [logPath, setLogPath] = useState<string | null>(null);
  const [checking, setChecking] = useState<CheckResult | "checking" | null>(null);
  useEffect(() => {
    api.appInfo().then((i) => {
      setVersion(i.version);
      setLogPath(i.logPath);
    });
  }, []);

  const lastFolder = settings.output.mode === "fixedFolder" ? settings.output.path : "";
  const chooseFolder = async () => {
    const dir = await open({ directory: true, defaultPath: lastFolder || undefined });
    if (typeof dir === "string") onChange({ ...settings, output: { mode: "fixedFolder", path: dir } });
  };

  return (
    <div className="fixed inset-0 z-20 flex justify-end bg-black/40" onClick={onClose}>
      <aside className="h-full w-full max-w-md space-y-6 overflow-y-auto border-l border-border bg-surface p-6" onClick={(e) => e.stopPropagation()}>
        <header className="flex items-center justify-between">
          <h2 className="text-lg font-bold">{t("settings.title")}</h2>
          <button onClick={onClose} className="rounded-md px-2 py-1 text-muted hover:bg-surface-2">✕</button>
        </header>

        <Section title={t("settings.output")}>
          {(["nextToSource", "fixedFolder", "askEachTime"] as const).map((mode) => (
            <label key={mode} className="flex cursor-pointer items-center gap-2 text-sm">
              <input
                type="radio"
                name="output"
                className="accent-[var(--color-accent)]"
                checked={settings.output.mode === mode}
                onChange={() => (mode === "fixedFolder" ? chooseFolder() : onChange({ ...settings, output: { mode } }))}
              />
              {t(`settings.output.${mode}`)}
            </label>
          ))}
          {settings.output.mode === "fixedFolder" && (
            <div className="flex items-center gap-2 pl-6">
              <span className="min-w-0 flex-1 truncate text-xs text-muted" title={settings.output.path}>
                {settings.output.path || t("settings.output.none")}
              </span>
              <button onClick={chooseFolder} className="rounded-md border border-border px-2 py-1 text-xs hover:border-accent">
                {t("settings.output.choose")}
              </button>
            </div>
          )}
        </Section>

        <Section title={t("settings.language")}>
          <Segmented value={settings.language} options={[["fr", "Français"], ["en", "English"]]} onChange={(language) => onChange({ ...settings, language })} />
        </Section>

        <Section title={t("settings.theme")}>
          <Segmented
            value={settings.theme}
            options={[["system", t("settings.theme.system")], ["dark", t("settings.theme.dark")], ["light", t("settings.theme.light")]]}
            onChange={(theme) => onChange({ ...settings, theme })}
          />
        </Section>

        <Section title="Windows">
          <label className="flex cursor-pointer items-center justify-between gap-4 text-sm">
            {t("settings.shell")}
            <input
              type="checkbox"
              className="h-4 w-4 accent-[var(--color-accent)]"
              checked={settings.shellIntegration}
              onChange={(e) => onChange({ ...settings, shellIntegration: e.target.checked })}
            />
          </label>
          <p className="text-xs text-muted">{t("settings.shell.hint")}</p>
          <label className={`flex cursor-pointer items-center justify-between gap-4 pt-2 text-sm ${settings.shellIntegration ? "" : "opacity-50"}`}>
            {t("settings.quiet")}
            <input
              type="checkbox"
              className="h-4 w-4 accent-[var(--color-accent)]"
              disabled={!settings.shellIntegration}
              checked={settings.quietContextMenu}
              onChange={(e) => onChange({ ...settings, quietContextMenu: e.target.checked })}
            />
          </label>
          <p className="text-xs text-muted">{t("settings.quiet.hint")}</p>
        </Section>

        <Section title={t("settings.modules")}>
          <ModulesList />
        </Section>

        <Section title={t("settings.updates")}>
          <label className="flex cursor-pointer items-center justify-between gap-4 text-sm">
            {t("settings.updates.auto")}
            <input
              type="checkbox"
              className="h-4 w-4 accent-[var(--color-accent)]"
              checked={settings.checkUpdates}
              onChange={(e) => onChange({ ...settings, checkUpdates: e.target.checked })}
            />
          </label>
          <div className="flex items-center gap-3">
            <button
              disabled={checking === "checking"}
              onClick={async () => {
                setChecking("checking");
                setChecking(await onCheckUpdate());
              }}
              className="rounded-md border border-border px-2 py-1 text-xs hover:border-accent disabled:opacity-50"
            >
              {t("settings.updates.check")}
            </button>
            {checking && <span className="text-xs text-muted">{t(`settings.updates.${checking}`)}</span>}
          </div>
        </Section>

        <Section title={t("settings.aboutTitle")}>
          <p className="text-sm">
            <span className="font-semibold">Convix</span> <span className="text-muted">{t("settings.about", { version })}</span>
          </p>
          <p className="text-xs text-muted">{t("settings.about.text")}</p>
          <div className="flex flex-wrap gap-2">
            <button onClick={() => openUrl("https://github.com/Luz0gg/convix")} className="rounded-md border border-border px-2 py-1 text-xs hover:border-accent">
              GitHub
            </button>
            <button onClick={() => openUrl("https://github.com/Luz0gg/convix/issues/new")} className="rounded-md border border-border px-2 py-1 text-xs hover:border-accent">
              {t("settings.about.bug")}
            </button>
            {logPath && (
              <button onClick={() => revealItemInDir(logPath)} className="rounded-md border border-border px-2 py-1 text-xs hover:border-accent">
                {t("settings.about.log")}
              </button>
            )}
          </div>
          <p className="text-[11px] text-muted">{t("settings.about.credits")}</p>
        </Section>
      </aside>
    </div>
  );
}
