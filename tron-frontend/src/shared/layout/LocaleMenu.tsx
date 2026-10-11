import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useLocale } from "@/i18n";

export function LocaleMenu() {
  const { t } = useTranslation();
  const { changeLocale, currentLocale, supportedLocales } = useLocale();
  const selected = supportedLocales.find((locale) => locale.code === currentLocale);

  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger render={<Button id="tron-locale-select" variant="ghost" className="h-9 w-full min-w-0 justify-start gap-2 px-2.5" />}>

        <span className="shrink-0">{t("common:locale.label")}</span>
        <span className="ml-auto min-w-0 truncate text-xs text-muted-foreground">{selected ? t(selected.labelKey) : currentLocale}</span>

      </DropdownMenuTrigger>
      <DropdownMenuContent data-tron-locale-menu side="right" align="end" sideOffset={8} className="w-56 max-w-[calc(100vw-2rem)]">
        <DropdownMenuRadioGroup value={currentLocale} onValueChange={(value) => {
          const locale = supportedLocales.find((item) => item.code === value);
          if (locale) void changeLocale(locale.code);
        }} aria-label={t("common:locale.label")}>
          {supportedLocales.map((locale) => (
            <DropdownMenuRadioItem key={locale.code} value={locale.code} className="min-h-9 break-words">
              {t(locale.labelKey)}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
