import { useTranslation } from "react-i18next";
import { Switch } from "@/components/ui/switch";
import { Item, ItemActions, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import { setAppEntranceAnimation, useAppEntranceAnimation } from "@/shared/motion/appEntrancePreferences";

export function AppEntranceSettings() {
  const { t } = useTranslation("pages");
  const enabled = useAppEntranceAnimation();
  return <SurfaceCard className="p-0">
    <Item size="xs">
      <ItemContent>
        <ItemTitle>{t("settingsAppearance.appEntranceAnimation")}</ItemTitle>
        <ItemDescription>{t("settingsAppearance.appEntranceAnimationDescription")}</ItemDescription>
      </ItemContent>
      <ItemActions>
        <Switch aria-label={t("settingsAppearance.appEntranceAnimation")} checked={enabled} onCheckedChange={setAppEntranceAnimation} />
      </ItemActions>
    </Item>
  </SurfaceCard>;
}
