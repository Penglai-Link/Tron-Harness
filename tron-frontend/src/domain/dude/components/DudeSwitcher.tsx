import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { AgentAvatar } from "@/domain/agent/components/AgentAvatarPicker";
import type { Dude } from "../api/dudeApi";

export function DudeSwitcher({ dudes, value, onValueChange }: {
  dudes: readonly Dude[];
  value: string;
  onValueChange: (code: string) => void;
}) {
  const { t } = useTranslation("pages");
  const selected = dudes.find(item => item.dudeCode === value);
  const role = (item: Dude) => t(item.kind === "Memory" ? "dude.memory" : item.isMain ? "dude.main" : "dude.custom");
  return <>
    <h1 className="min-w-0">
      <DropdownMenu modal={false}>
        <DropdownMenuTrigger
          aria-label={selected ? `${t("dude.selectDude")}: ${selected.name}` : t("dude.selectDude")}
          render={<Button variant="ghost" className="h-auto max-w-full gap-2 px-1 py-1 text-base font-medium" />}
        >
          {selected ? <AgentAvatar avatarId={selected.avatarId} className="size-7 shrink-0 rounded-md" /> : null}
          <span className="min-w-0 truncate">{selected?.name ?? t("dude.selectDude")}</span>
          <ChevronDown aria-hidden="true" className="size-3.5 shrink-0 text-[var(--color-on-surface-variant)]" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-80 max-w-[calc(100vw-2rem)]" aria-label={t("dude.selectDude")}>
          <DropdownMenuRadioGroup value={value} onValueChange={next => onValueChange(next)} aria-label={t("dude.selectDude")}>
            {dudes.map(item => <DropdownMenuRadioItem key={item.dudeCode} value={item.dudeCode} closeOnClick className="gap-3 py-2">
              <AgentAvatar avatarId={item.avatarId} className="size-8 shrink-0 rounded-md" />
              <span className="min-w-0 flex-1">
                <span className="block break-words font-medium">{item.name}</span>
                <span className="text-xs text-[var(--color-on-surface-variant)]">{role(item)}</span>
              </span>
              <Badge variant={item.enabled ? "success" : "outline"}>{t(item.enabled ? "dude.activated" : "dude.paused")}</Badge>
            </DropdownMenuRadioItem>)}
          </DropdownMenuRadioGroup>
          {dudes.length ? <DropdownMenuSeparator /> : null}
          <DropdownMenuItem render={<Link to="/settings/agents?tab=dudes" />} className="min-h-9">
            {t("dude.manage")}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </h1>
    {selected ? <>
      <Badge variant="outline">{role(selected)}</Badge>
      <Badge variant={selected.enabled ? "success" : "outline"}>{t(selected.enabled ? "dude.activated" : "dude.paused")}</Badge>
    </> : null}
  </>;
}
