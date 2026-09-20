import { useTranslation } from "react-i18next";

import { Field, TextInput } from "@/components/forms";
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
import { UpdateSettingsCard } from "@/domain/update/UpdateSettingsCard";

export function SettingsGeneralPage() {
  const { t: shellT } = useTranslation("shell");
  const { t: pageT } = useTranslation("pages");
  const {
    preferences: aguiWorkReplayPreferences,
    setRecentTurnLimit,
  } = useAguiWorkReplayPreferences();

  return (
    <>
      <PageHeader title={shellT("settings.items.general")} />

      <div className="grid gap-4 xl:grid-cols-2">
        <UpdateSettingsCard />

        <SystemConfigTable />

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
        </SurfaceCard>
      </div>
    </>
  );
}
