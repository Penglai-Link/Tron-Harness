import { ChevronRight, Palette } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useToast } from "@/components/ui/toast";
import { useTheme } from "@/shared/theme/ThemeProvider";

export function ThemeMenu() {
  const { t } = useTranslation();
  const { skins, activeSkin, loading, selectSkin } = useTheme();
  const toast = useToast();

  async function select(id: string) {
    try { await selectSkin(id); }
    catch (reason) {
      toast.error(reason instanceof Error ? reason.message : t("pages:settingsAppearance.skinFailed"));
    }
  }

  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger render={<Button variant="ghost" className="w-full justify-start" />}>
        <Palette aria-hidden="true" />
        <span>{t("shell:theme.label")}</span>
        <ChevronRight aria-hidden="true" className="ml-auto" />
      </DropdownMenuTrigger>
      <DropdownMenuContent data-tron-theme-menu side="right" align="end" sideOffset={8} className="w-56 max-w-[calc(100vw-2rem)]">
        <DropdownMenuRadioGroup value={activeSkin.id} onValueChange={(id) => void select(id)} aria-label={t("shell:theme.label")}>
          {skins.map((skin) => (
            <DropdownMenuRadioItem key={skin.id} value={skin.id} disabled={loading} closeOnClick={false} className="min-h-9 break-words">
              {skin.id === "light" ? t("shell:theme.options.light")
                : skin.id === "matrix-dark" ? t("shell:theme.options.matrixDark") : skin.name}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
