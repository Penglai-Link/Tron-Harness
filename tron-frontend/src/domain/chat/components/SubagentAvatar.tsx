import { Check, CircleHelp, Pause, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { HistoryStatus } from "@eliteforge/agent-ui-chat-core";
import { AgentAvatar } from "@/domain/agent/components/AgentAvatarPicker";
import "./SubagentAvatar.css";

export function SubagentAvatar({ avatarId, name, status }: { avatarId?: string | null; name: string; status: HistoryStatus }) {
  const { t } = useTranslation("pages");
  const label = `${name} · ${t(`chat.subagents.statuses.${status}`)}`;
  const Mark = status === "Completed" || status === "Approved" ? Check
    : status === "Failed" ? X : status === "Interrupted" ? Pause : status === "Unknown" ? CircleHelp : null;
  return <span className="subagent-avatar" data-status={status} role="img" aria-label={label} title={label}>
    <span className="subagent-avatar__ring" aria-hidden="true" />
    <AgentAvatar avatarId={avatarId ?? undefined} className="subagent-avatar__image" />
    {Mark ? <span className="subagent-avatar__mark" aria-hidden="true"><Mark size={10} strokeWidth={2.5} /></span> : null}
  </span>;
}
