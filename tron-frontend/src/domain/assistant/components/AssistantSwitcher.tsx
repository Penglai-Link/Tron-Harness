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
import type { Assistant } from "../api/assistantApi";

export function AssistantSwitcher({ assistants, value, onValueChange }: {
  assistants: readonly Assistant[];
  value: string;
  onValueChange: (code: string) => void;
}) {
  const { t } = useTranslation("pages");
  const selected = assistants.find(item => item.assistantCode === value);
  const role = (item: Assistant) => t(item.kind === "Memory" ? "assistant.memory" : item.isMain ? "assistant.main" : "assistant.custom");
  return <>
    <h1 className="min-w-0">
      <DropdownMenu modal={false}>
        <DropdownMenuTrigger
          aria-label={selected ? `${t("assistant.selectAssistant")}: ${selected.name}` : t("assistant.selectAssistant")}
          render={<Button variant="ghost" className="h-auto max-w-full gap-2 px-1 py-1 text-base font-medium" />}
        >
          {selected ? <AgentAvatar avatarId={selected.avatarId} className="size-7 shrink-0 rounded-md" /> : null}
          <span className="min-w-0 truncate">{selected?.name ?? t("assistant.selectAssistant")}</span>
          <ChevronDown aria-hidden="true" className="size-3.5 shrink-0 text-[var(--color-on-surface-variant)]" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-80 max-w-[calc(100vw-2rem)]" aria-label={t("assistant.selectAssistant")}>
          <DropdownMenuRadioGroup value={value} onValueChange={next => onValueChange(next)} aria-label={t("assistant.selectAssistant")}>
            {assistants.map(item => <DropdownMenuRadioItem key={item.assistantCode} value={item.assistantCode} closeOnClick className="gap-3 py-2">
              <AgentAvatar avatarId={item.avatarId} className="size-8 shrink-0 rounded-md" />
              <span className="min-w-0 flex-1">
                <span className="block break-words font-medium">{item.name}</span>
                <span className="text-xs text-[var(--color-on-surface-variant)]">{role(item)}</span>
              </span>
              <Badge variant={item.enabled ? "success" : "outline"}>{t(item.enabled ? "assistant.activated" : "assistant.paused")}</Badge>
            </DropdownMenuRadioItem>)}
          </DropdownMenuRadioGroup>
          {assistants.length ? <DropdownMenuSeparator /> : null}
          <DropdownMenuItem render={<Link to="/settings/agents?tab=assistants" />} className="min-h-9">
            {t("assistant.manage")}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </h1>
    {selected ? <>
      <Badge variant="outline">{role(selected)}</Badge>
      <Badge variant={selected.enabled ? "success" : "outline"}>{t(selected.enabled ? "assistant.activated" : "assistant.paused")}</Badge>
    </> : null}
  </>;
}
