import { useEffect, useMemo, useState } from "react";
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

  const [turnLimitDraft, setTurnLimitDraft] = useState(String(aguiWorkReplayPreferences.recentTurnLimit));
  useEffect(() => setTurnLimitDraft(String(aguiWorkReplayPreferences.recentTurnLimit)), [aguiWorkReplayPreferences.recentTurnLimit]);

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
              type="text"
              value={turnLimitDraft}
              onChange={(event) => {
                const raw = event.target.value;
                if (!/^\d*$/.test(raw)) return;
                setTurnLimitDraft(raw);
                const number = Number(raw);
                if (raw && number >= minimumAguiWorkReplayTurnLimit && number <= maximumAguiWorkReplayTurnLimit) setRecentTurnLimit(number);
              }}
            />
          </Field>

          <div className="flex items-center justify-between gap-4 rounded-lg border border-[var(--color-outline-variant)] p-3">
            <span className="min-w-0 space-y-1">
              <span className="flex items-center gap-1 text-sm font-medium text-[var(--color-on-surface)]">
                {pageT("settingsGeneral.aguiWorkReplay.showReasoningCards")}
                <InfoTooltip ariaLabel={pageT("settingsGeneral.aguiWorkReplay.showReasoningCards")}>{pageT("settingsGeneral.aguiWorkReplay.showReasoningCardsHint")}</InfoTooltip>
              </span>
            </span>
            <Switch
              aria-label={pageT("settingsGeneral.aguiWorkReplay.showReasoningCards")}
              checked={aguiWorkReplayPreferences.showReasoningCards}
              onCheckedChange={setShowReasoningCards}
            />
          </div>

          <fieldset className="space-y-2 rounded-lg border border-[var(--color-outline-variant)] p-3">
            <legend className="sr-only">{pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitMode")}</legend>
            <div className="flex items-center gap-1 text-sm font-medium text-[var(--color-on-surface)]">
              {pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitMode")}
              <InfoTooltip ariaLabel={pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitMode")}>{pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitModeHint")}</InfoTooltip>
            </div>
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
