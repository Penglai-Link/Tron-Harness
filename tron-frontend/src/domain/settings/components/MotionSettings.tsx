import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import { MotionReveal } from "@/shared/motion";
import { setMotionPreference, useMotionPreference, type MotionPreference } from "@/shared/motion/motionPreferences";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Item, ItemActions, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item";
import { OptionSelect } from "@/components/option-select/OptionSelect";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";

export function MotionSettings() {
  const { t } = useTranslation("pages");
  const mode = useMotionPreference();
  const [replay, setReplay] = useState(0);
  const [selectedTab, setSelectedTab] = useState("all");
  return <SurfaceCard className="p-0">
    <Item size="xs">
      <ItemContent>
        <ItemTitle>{t("settingsAppearance.motion")}</ItemTitle>
        <ItemDescription>{t("settingsAppearance.motionDescription")}</ItemDescription>
      </ItemContent>
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
    <div className="space-y-3 border-t border-[var(--color-outline-variant)]/50 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-medium">{t("settingsAppearance.motionPreview")}</p>
        <Button size="sm" variant="outline" onClick={() => setReplay((value) => value + 1)}>
          {t("settingsAppearance.motionReplay")}
        </Button>
      </div>
      <div className="overflow-hidden rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-background)] p-3">
        <MotionReveal replayKey={replay} className="space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <Tabs value={selectedTab} onValueChange={(value) => setSelectedTab(value ?? "all")}>
              <TabsList aria-label={t("settingsAppearance.motionTabs")}>
                <TabsTrigger value="all">{t("settingsAppearance.motionAll")}</TabsTrigger>
                <TabsTrigger value="active">{t("settingsAppearance.motionActive")}</TabsTrigger>
                <TabsTrigger value="done">{t("settingsAppearance.motionDone")}</TabsTrigger>
              </TabsList>
            </Tabs>
            <Button size="sm">{t("settingsAppearance.motionAction")}</Button>
          </div>
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-[var(--color-outline-variant)] px-3 py-2">
            <div className="flex min-w-0 items-center gap-2">
              <span className="truncate text-sm">{t("settingsAppearance.motionTask")}</span>
              <Badge variant={selectedTab === "done" ? "success" : "info"}>
                {selectedTab === "done" ? t("settingsAppearance.motionDone") : t("settingsAppearance.motionActive")}
              </Badge>
            </div>
            <Button size="sm" variant="outline">{t("settingsAppearance.motionDetails")}</Button>
          </div>
          <div className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto]">
            <Input aria-label={t("settingsAppearance.motionInput")} placeholder={t("settingsAppearance.motionInput")} />
            <Button variant="secondary">{t("settingsAppearance.motionSave")}</Button>
          </div>
        </MotionReveal>
      </div>
    </div>
  </SurfaceCard>;
}
