import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import { MotionReveal, ShinyText } from "@/shared/motion";
import { setMotionPreference, useMotionPreference, type MotionPreference } from "@/shared/motion/motionPreferences";
import { Button } from "@/components/ui/button";
import { Item, ItemActions, ItemContent, ItemTitle } from "@/components/ui/item";
import { OptionSelect } from "@/components/option-select/OptionSelect";

export function MotionSettings() {
  const { t } = useTranslation("pages");
  const mode = useMotionPreference();
  const [replay, setReplay] = useState(0);
  return <SurfaceCard className="p-0">
    <Item size="xs">
      <ItemContent><ItemTitle>{t("settingsAppearance.motion")}</ItemTitle></ItemContent>
      <ItemActions>
        <div className="w-40 max-w-full">
          <OptionSelect ariaLabel={t("settingsAppearance.motion")} value={mode}
            onChange={(value) => setMotionPreference(value as MotionPreference)}
            options={[
              { value: "system", label: t("settingsAppearance.motionSystem") },
              { value: "reduced", label: t("settingsAppearance.motionReduced") },
              { value: "off", label: t("settingsAppearance.motionOff") },
            ]} />
        </div>
      </ItemActions>
    </Item>
    <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)]/50 px-3 py-3">
      <MotionReveal replayKey={replay} className="text-sm text-[var(--color-on-surface-variant)]">
        <ShinyText>{t("settingsAppearance.motionPreview")}</ShinyText>
      </MotionReveal>
      <Button size="sm" variant="outline" onClick={() => setReplay((value) => value + 1)}>
        {t("settingsAppearance.motionReplay")}
      </Button>
    </div>
  </SurfaceCard>;
}
