import { ChevronRight } from "lucide-react";
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

  const label = (skin: typeof activeSkin) => skin.id === "light" ? t("shell:theme.options.light")
    : skin.id === "matrix-dark" ? t("shell:theme.options.matrixDark") : skin.name;

  async function select(id: string) {
    try { await selectSkin(id); }
    catch (reason) {
      toast.error(reason instanceof Error ? reason.message : t("pages:settingsAppearance.skinFailed"));
    }
  }

  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger render={<Button variant="ghost" className="h-9 w-full min-w-0 justify-start gap-2 px-2.5" />}>

        <span className="shrink-0">{t("shell:theme.label")}</span>
        <span className="ml-auto min-w-0 truncate text-xs text-muted-foreground" title={label(activeSkin)}>{label(activeSkin)}</span>
        <ChevronRight aria-hidden="true" className="size-3.5 shrink-0" />
      </DropdownMenuTrigger>
      <DropdownMenuContent data-tron-theme-menu side="right" align="end" sideOffset={8} className="w-56 max-w-[calc(100vw-2rem)]">
        <DropdownMenuRadioGroup value={activeSkin.id} onValueChange={(id) => void select(id)} aria-label={t("shell:theme.label")}>
          {skins.map((skin) => (
            <DropdownMenuRadioItem key={skin.id} value={skin.id} disabled={loading} closeOnClick={false} className="min-h-9 break-words">
              {label(skin)}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
