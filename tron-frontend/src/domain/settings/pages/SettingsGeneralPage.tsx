import { useTranslation } from "react-i18next";

import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextInput,
  useTheme,
} from "@/shared";
import { useLocale } from "@/i18n";
import {
  aguiDebugModeOptions,
  aguiEventTypeOptions,
  resolveAguiDebugEventTypes,
  useAguiDebugPreferences,
  type AguiDebugMode,
} from "@/domain/settings/model/aguiDebugPreferences";
import {
  maximumAguiWorkReplayTurnLimit,
  minimumAguiWorkReplayTurnLimit,
  useAguiWorkReplayPreferences,
} from "@/domain/settings/model/aguiWorkReplayPreferences";
import { SelectItem } from "@/components/ui/select";

export function SettingsGeneralPage() {
  const { t } = useTranslation(["common", "shell"]);
  const { changeLocale, currentLocale, supportedLocales } = useLocale();
  const { setTheme, theme, themes } = useTheme();
  const {
    preferences: aguiDebugPreferences,
    setMode: setAguiDebugMode,
    toggleCustomEventType,
  } = useAguiDebugPreferences();
  const {
    preferences: aguiWorkReplayPreferences,
    setRecentTurnLimit,
  } = useAguiWorkReplayPreferences();
  const readOnlyAguiCardList = aguiDebugPreferences.mode !== "custom";
  const selectedEventTypes = new Set(resolveAguiDebugEventTypes(aguiDebugPreferences));
  const selectedMode = aguiDebugModeOptions.find((option) => option.value === aguiDebugPreferences.mode) ?? aguiDebugModeOptions[0];

  return (
    <>
      <PageHeader title={t("shell:settings.items.general")} />

      <div className="grid gap-4 xl:grid-cols-2">
        <SurfaceCard className="space-y-3">
          <label className="grid gap-1.5">
            <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
              {t("shell:theme.label")}
            </span>
            <SelectInput
              value={theme}
              onValueChange={(event) => setTheme(event === "matrix-dark" ? "matrix-dark" : "light")}
            >
              {themes.map((themeOption) => (
                <SelectItem key={themeOption.value} value={themeOption.value}>
                  {t(themeOption.labelKey)}
                </SelectItem>
              ))}
            </SelectInput>
          </label>
          <p className="text-sm text-[var(--color-on-surface-variant)]">{t("shell:theme.hint")}</p>
        </SurfaceCard>

        <SurfaceCard className="space-y-3">
          <label className="grid gap-1.5">
            <span className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
              {t("common:locale.label")}
            </span>
            <SelectInput
              value={currentLocale}
              onValueChange={(event) => void changeLocale(event as typeof currentLocale)}
            >
              {supportedLocales.map((locale) => (
                <SelectItem key={locale.code} value={locale.code}>
                  {t(locale.labelKey)}
                </SelectItem>
              ))}
            </SelectInput>
          </label>
          <p className="text-sm text-[var(--color-on-surface-variant)]">{t("common:locale.unavailable")}</p>
        </SurfaceCard>

        <SurfaceCard className="space-y-4 xl:col-span-2">
          <div>
            <h2 className="text-sm font-semibold text-[var(--color-on-surface)]">AG-UI Debug</h2>
            <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
              Controls which AG-UI event cards are rendered in chat transcripts.
            </p>
          </div>

          <Field label="Event Cards">
            <SelectInput
              aria-label="AG-UI event card visibility"
              value={aguiDebugPreferences.mode}
              onValueChange={(event) => setAguiDebugMode(event as AguiDebugMode)}
            >
              {aguiDebugModeOptions.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectInput>
          </Field>
          <p className="text-sm text-[var(--color-on-surface-variant)]">{selectedMode.description}</p>

          <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3" aria-label="AG-UI event card types">
            {aguiEventTypeOptions.map((option) => (
              <label
                className="flex min-h-16 items-start gap-3 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm"
                key={option.value}
              >
                <input
                  className="mt-1 size-4 accent-[var(--color-primary)] disabled:cursor-not-allowed"
                  type="checkbox"
                  checked={selectedEventTypes.has(option.value)}
                  disabled={readOnlyAguiCardList}
                  onChange={(event) => toggleCustomEventType(option.value, event.target.checked)}
                />
                <span className="min-w-0">
                  <span className="block font-medium text-[var(--color-on-surface)]">{option.label}</span>
                  <span className="mt-0.5 block text-xs text-[var(--color-on-surface-variant)]">{option.description}</span>
                </span>
              </label>
            ))}
          </div>
        </SurfaceCard>

        <SurfaceCard className="space-y-4 xl:col-span-2">
          <div>
            <h2 className="text-sm font-semibold text-[var(--color-on-surface)]">AG-UI Work Replay</h2>
            <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
              Controls how many recent completed conversation turns keep user-facing work rails after chat switching or page refresh.
            </p>
          </div>

          <Field
            label="Recent Work Turns"
            hint={`Show between ${minimumAguiWorkReplayTurnLimit} and ${maximumAguiWorkReplayTurnLimit} recent turns per session.`}
          >
            <TextInput
              aria-label="AG-UI recent work turn limit"
              inputMode="numeric"
              max={maximumAguiWorkReplayTurnLimit}
              min={minimumAguiWorkReplayTurnLimit}
              type="number"
              value={aguiWorkReplayPreferences.recentTurnLimit}
              onChange={(event) => setRecentTurnLimit(Number(event.target.value))}
            />
          </Field>
        </SurfaceCard>
      </div>
    </>
  );
}
