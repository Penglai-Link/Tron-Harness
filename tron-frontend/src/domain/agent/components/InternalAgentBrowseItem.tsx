import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { AgentAvatar } from "./AgentAvatarPicker";
import type { WorkspaceInternalAgentRecord } from "../model";

export function InternalAgentBrowseItem({ agent, name, groupLabel, disabled, canDelete = false, onOpen, onDelete }: {
  agent: WorkspaceInternalAgentRecord; name: string; groupLabel: string; disabled: boolean; canDelete?: boolean; onOpen: () => void; onDelete?: () => void;
}) {
  const { t } = useTranslation(["pages"]);
  const builtInLabel = t("pages:workspaceAgents.internal.groups.builtIn", { defaultValue: "Built-in agent" });
  const action = t("pages:workspaceAgents.internal.editAgent", { defaultValue: "Edit {{name}}", name });
  return <div className="relative min-w-0">
    <Button variant="ghost" disabled={disabled} aria-label={agent.builtIn ? `${action} · ${builtInLabel}` : action}
      className="h-auto w-full min-w-0 flex-col gap-2 rounded-xl px-2 py-4 text-center whitespace-normal" onClick={onOpen}>
      <AgentAvatar avatarId={agent.avatarId} className="size-20 shrink-0 rounded-full" />
      <span className="block w-full min-w-0">
        <span className="block truncate text-sm font-medium" title={name}>{name}</span>
        <span className="mt-1 block truncate font-mono text-xs text-[var(--color-on-surface-variant)]" title={agent.agentCode}>{agent.agentCode}</span>
      </span>
      <span className="max-w-full truncate text-xs font-normal text-[var(--color-on-surface-variant)]" title={groupLabel}>{groupLabel}</span>
    </Button>
    {canDelete && onDelete ? <Button type="button" variant="ghost" size="sm" disabled={disabled}
      aria-label={t("pages:workspaceAgents.internal.groups.deleteAgent", { defaultValue: "Delete {{name}}", name })}
      title={t("pages:workspaceAgents.internal.groups.deleteAgent", { defaultValue: "Delete {{name}}", name })}
      className="absolute right-1 top-1 text-[var(--color-error)]" onClick={onDelete}>{t("pages:workspaceAgents.internal.groups.deleteAgent", { defaultValue: "Delete {{name}}", name })}</Button> : null}
    {agent.builtIn ? <Tooltip>
      <TooltipTrigger aria-label={builtInLabel}
        className="absolute left-1 top-1 flex h-6 px-1 text-xs items-center justify-center rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] text-[var(--color-on-surface-variant)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)]"
>
        {builtInLabel}
      </TooltipTrigger>
      <TooltipContent>{builtInLabel}</TooltipContent>
    </Tooltip> : null}
  </div>;
}
