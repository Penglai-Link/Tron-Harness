import { useMemo, useState } from "react";
import { ChevronRight, Globe } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useUpdateUserTimeZone } from "../hooks/useUpdateUserTimeZone";
export function TimeZoneMenu() {
  const { t } = useTranslation("shell");
  const { timeZone, setTimeZone, saving } = useUpdateUserTimeZone();
  const [search, setSearch] = useState("");
  const zones = useMemo(() => {
    const supported = Intl as typeof Intl & {
      supportedValuesOf?: (key: "timeZone") => string[];
    };
    return [
      ...new Set([
        "Asia/Shanghai",
        "UTC",
        ...(supported.supportedValuesOf?.("timeZone") ?? [
          "Asia/Tokyo",
          "Asia/Singapore",
          "Europe/London",
          "Europe/Paris",
          "America/New_York",
          "America/Los_Angeles",
        ]),
      ]),
    ];
  }, []);
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger
        render={
          <Button
            disabled={saving}
            variant="ghost"
            className="h-9 w-full min-w-0 justify-start gap-2 px-2.5"
          />
        }
      >
        <Globe aria-hidden="true" />
        <span>{t("userMenu.timeZone")}</span>
        <span className="ml-auto min-w-0 truncate text-xs text-muted-foreground">
          {timeZone}
        </span>
        <ChevronRight aria-hidden="true" className="size-3.5 shrink-0" />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        data-tron-timezone-menu
        side="right"
        align="end"
        sideOffset={8}
        className="w-72 max-w-[calc(100vw-2rem)]"
      >
        <div className="p-2">
          <Input
            aria-label={t("userMenu.searchTimeZone")}
            placeholder={t("userMenu.searchTimeZone")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
          />
        </div>
        <DropdownMenuRadioGroup
          className="max-h-64 overflow-y-auto"
          value={timeZone}
          onValueChange={setTimeZone}
          aria-label={t("userMenu.timeZone")}
        >
          {zones
            .filter((zone) => zone.toLowerCase().includes(search.toLowerCase()))
            .map((zone) => (
              <DropdownMenuRadioItem key={zone} value={zone} disabled={saving}>
                {zone}
              </DropdownMenuRadioItem>
            ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
