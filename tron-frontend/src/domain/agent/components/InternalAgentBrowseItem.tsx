import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { AgentAvatar } from "./AgentAvatarPicker";
import type { WorkspaceInternalAgentRecord } from "../model";

export function InternalAgentBrowseItem({ agent, name, disabled, onOpen }: {
  agent: WorkspaceInternalAgentRecord; name: string; disabled: boolean; onOpen: () => void;
}) {
  const { t } = useTranslation(["pages"]);
  const [tooltipOpen, setTooltipOpen] = useState(false);
  const builtInLabel = t("pages:workspaceAgents.internal.groups.builtIn", { defaultValue: "Built-in agent" });
  const action = t(agent.builtIn ? "pages:workspaceAgents.internal.groups.view" : "pages:workspaceAgents.internal.editAgent", { defaultValue: agent.builtIn ? "View {{name}}" : "Edit {{name}}", name });
  return <div className="relative min-w-0">
    <Button variant="ghost" disabled={disabled} aria-label={agent.builtIn ? `${action} · ${builtInLabel}` : action}
      className="h-auto w-full min-w-0 flex-col gap-2 rounded-xl px-2 py-3 text-center" onClick={onOpen}>
      <AgentAvatar avatarId={agent.avatarId} className="size-24 shrink-0 rounded-full" />
      <span className="block w-full min-w-0">
        <span className="block truncate text-sm font-medium" title={name}>{name}</span>
        <span className="mt-1 block truncate font-mono text-xs text-[var(--color-on-surface-variant)]" title={agent.agentCode}>{agent.agentCode}</span>
      </span>
    </Button>
    {agent.builtIn ? <Tooltip open={tooltipOpen} onOpenChange={setTooltipOpen}>
      <TooltipTrigger aria-label={builtInLabel} delay={0}
        className="absolute left-[calc(50%+1.5rem)] top-[5.25rem] flex size-6 items-center justify-center rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] text-[var(--color-on-surface-variant)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)]"
        onFocus={() => setTooltipOpen(true)} onBlur={() => setTooltipOpen(false)}>
        <ShieldCheck size={14} aria-hidden="true" />
      </TooltipTrigger>
      <TooltipContent>{builtInLabel}</TooltipContent>
    </Tooltip> : null}
  </div>;
}
