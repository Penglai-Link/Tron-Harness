import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Field, SelectInput, TextInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
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
import { RemoteAccessSettings } from "@/domain/settings/components/RemoteAccessSettings";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";
import { UpdateSettingsCard } from "@/domain/update/UpdateSettingsCard";
import { WebVersionCard } from "@/domain/update/WebVersionCard";
import { BrowserNotificationSettingsCard } from "@/domain/notification/components/BrowserNotificationSettingsCard";
import { BackendEnvironmentSettings } from "@/domain/settings/components/BackendEnvironmentSettings";

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
    <div className="mx-auto w-full max-w-6xl px-1 py-2 sm:px-3 lg:px-5">
      <PageHeader title={shellT("settings.items.general")} />

      <div className="flex flex-col divide-y divide-[var(--color-outline-variant)]">
        <div>{desktopBridge.isDesktop() ? <UpdateSettingsCard /> : <WebVersionCard />}</div>
        {desktopBridge.isElectron() && <div><BackendEnvironmentSettings /></div>}

        <div><SystemConfigTable /></div>

        <div><RemoteAccessSettings /></div>

        <div><BrowserNotificationSettingsCard /></div>

        <div><SurfaceCard className="space-y-4 xl:col-span-2">
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

          <div className="flex items-center justify-between gap-4 border-t border-[var(--color-outline-variant)] py-4">
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

          <fieldset className="space-y-2 border-t border-[var(--color-outline-variant)] py-4">
            <legend className="sr-only">{pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitMode")}</legend>
            <div className="flex items-center gap-1 text-sm font-medium text-[var(--color-on-surface)]">
              {pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitMode")}
              <InfoTooltip ariaLabel={pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitMode")}>{pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitModeHint")}</InfoTooltip>
            </div>
            <SelectInput
              aria-label={pageT("settingsGeneral.aguiWorkReplay.runningEnterSubmitMode")}
              value={aguiWorkReplayPreferences.runningEnterSubmitMode}
              onValueChange={(value) => { if (value === "steer" || value === "queue") setRunningEnterSubmitMode(value); }}
            >
              {(["steer", "queue"] as const).map((mode) => (
                <SelectItem key={mode} value={mode}>
                  {pageT(`settingsGeneral.aguiWorkReplay.${mode}OnEnter`)}
                </SelectItem>
              ))}
            </SelectInput>
          </fieldset>
        </SurfaceCard></div>
      </div>
    </div>
  );
}
