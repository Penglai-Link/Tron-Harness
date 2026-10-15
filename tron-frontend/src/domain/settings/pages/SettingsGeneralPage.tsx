import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { Field, TextInput } from "@/components/forms";
import { Switch } from "@/components/ui/switch";
import { InfoTooltip } from "@/components/info-tooltip";
import {
  PageHeader,
  SurfaceCard,
} from "@/shared";
import {
  maximumAguiWorkReplayTurnLimit,
  minimumAguiWorkReplayTurnLimit,
  useAguiWorkReplayPreferences,
} from "@/domain/settings/model/aguiWorkReplayPreferences";
import { SystemConfigTable } from "@/domain/settings/components/SystemConfigTable";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";
import { UpdateSettingsCard } from "@/domain/update/UpdateSettingsCard";
import { WebVersionCard } from "@/domain/update/WebVersionCard";
import { BrowserNotificationSettingsCard } from "@/domain/notification/components/BrowserNotificationSettingsCard";

export function SettingsGeneralPage() {
  const { t: shellT } = useTranslation("shell");
  const { t: pageT } = useTranslation("pages");
  const desktopBridge = useMemo(() => new DesktopBridge(), []);
  const {
    preferences: aguiWorkReplayPreferences,
    setRecentTurnLimit,
    setShowReasoningCards,
    setRunningEnterSubmitMode,
  } = useAguiWorkReplayPreferences();

  return (
    <>
      <PageHeader title={shellT("settings.items.general")} />

      <div className="grid gap-4 xl:grid-cols-2">
        {desktopBridge.isDesktop() ? <UpdateSettingsCard /> : <WebVersionCard />}

        <SystemConfigTable />

        <BrowserNotificationSettingsCard />

        <SurfaceCard className="space-y-4 xl:col-span-2">
          <div className="flex items-center gap-1.5">
            <h2 className="text-sm font-semibold text-[var(--color-on-surface)]">{pageT("settingsGeneral.aguiWorkReplay.title")}</h2>
            <InfoTooltip ariaLabel={pageT("settingsGeneral.aguiWorkReplay.help")}>{pageT("settingsGeneral.aguiWorkReplay.description")}</InfoTooltip>
          </div>

          <Field
            label={pageT("settingsGeneral.aguiWorkReplay.recentWorkTurns")}
            hint={pageT("settingsGeneral.aguiWorkReplay.recentWorkTurnsHint", {
              min: minimumAguiWorkReplayTurnLimit,
              max: maximumAguiWorkReplayTurnLimit,
            })}
          >
            <TextInput
              aria-label={pageT("settingsGeneral.aguiWorkReplay.recentWorkTurnsAria")}
              inputMode="numeric"
              max={maximumAguiWorkReplayTurnLimit}
              min={minimumAguiWorkReplayTurnLimit}
              type="number"
              value={aguiWorkReplayPreferences.recentTurnLimit}
              onChange={(event) => setRecentTurnLimit(Number(event.target.value))}
            />
          </Field>

          <label className="flex items-center justify-between gap-4 rounded-lg border border-[var(--color-outline-variant)] p-3">
            <span className="min-w-0 space-y-1">
              <span className="block text-sm font-medium text-[var(--color-on-surface)]">{pageT("settingsGeneral.aguiWorkReplay.showReasoningCards")}</span>
              <span className="block text-xs text-[var(--color-on-surface-variant)]">{pageT("settingsGeneral.aguiWorkReplay.showReasoningCardsHint")}</span>
            </span>
            <Switch
              aria-label={pageT("settingsGeneral.aguiWorkReplay.showReasoningCards")}
              checked={aguiWorkReplayPreferences.showReasoningCards}
              onCheckedChange={setShowReasoningCards}
            />
          </label>

          <fieldset className="space-y-2 rounded-lg border border-[var(--color-outline-variant)] p-3">
            <legend className="px-1 text-sm font-medium text-[var(--color-on-surface)]">{pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitMode")}</legend>
            <p className="text-xs text-[var(--color-on-surface-variant)]">{pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitModeHint")}</p>
            <div className="flex flex-wrap gap-4">
              {(["steer", "queue"] as const).map((mode) => (
                <label key={mode} className="flex cursor-pointer items-center gap-2 text-sm text-[var(--color-on-surface)]">
                  <input
                    type="radio"
                    name="runningEnterSubmitMode"
                    value={mode}
                    checked={aguiWorkReplayPreferences.runningEnterSubmitMode === mode}
                    onChange={() => setRunningEnterSubmitMode(mode)}
                  />
                  {pageT(`settingsGeneral.aguiWorkReplay.${mode}OnEnter`)}
                </label>
              ))}
            </div>
          </fieldset>
        </SurfaceCard>
      </div>
    </>
  );
}
