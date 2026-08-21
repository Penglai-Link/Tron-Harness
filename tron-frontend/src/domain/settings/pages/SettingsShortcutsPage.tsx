import { useTranslation } from "react-i18next";

import { PageHeader, SurfaceCard } from "@/shared";

const platformShortcuts = [
  {
    key: "openTerminal",
  },
  {
    key: "newSession",
  },
  {
    key: "closeCurrentTab",
  },
] as const;

export function SettingsShortcutsPage() {
  const { t } = useTranslation("pages");
  const pageT = (key: string) => t(`settingsShortcuts.${key}`);

  return (
    <div className="space-y-6">
      <PageHeader title={pageT("title")} />

      <SurfaceCard className="space-y-5">
        <h2 className="text-sm font-semibold text-[var(--color-on-surface)]">{pageT("supportedTitle")}</h2>

        <div className="overflow-x-auto">
          <div className="min-w-[760px] overflow-hidden rounded-xl border border-[var(--color-outline-variant)]">
            <div className="grid grid-cols-[minmax(180px,1fr)_minmax(260px,1.5fr)_140px] gap-4 bg-[var(--color-surface-container)] px-4 py-3 text-xs font-semibold text-[var(--color-on-surface-variant)]">
              <span>{pageT("columns.action")}</span>
              <span>{pageT("columns.description")}</span>
              <span>{pageT("columns.shortcut")}</span>
            </div>
            <div className="divide-y divide-[var(--color-outline-variant)]">
              {platformShortcuts.map(({ key }) => (
                <div className="grid grid-cols-[minmax(180px,1fr)_minmax(260px,1.5fr)_140px] gap-4 px-4 py-4 text-sm" key={key}>
                  <span className="font-medium text-[var(--color-on-surface)]">{pageT(`shortcuts.${key}.label`)}</span>
                  <span className="text-[var(--color-on-surface-variant)]">{pageT(`shortcuts.${key}.description`)}</span>
                  <code className="text-[var(--color-on-surface-variant)]">{pageT("unassigned")}</code>
                </div>
              ))}
            </div>
          </div>
        </div>
      </SurfaceCard>
    </div>
  );
}
