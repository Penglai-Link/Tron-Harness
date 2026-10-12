import { AgentAvatar } from "./AgentAvatarPicker";
import "./AgentAvatarWithActivity.css";

export function AgentAvatarWithActivity({
  active,
  avatarId,
  name,
}: {
  active: boolean;
  avatarId?: string;
  name: string;
}) {
  return (
    <span className="agent-activity-avatar" data-active={active} aria-hidden="true">
      <span className="agent-activity-avatar__ring" />
      <AgentAvatar avatarId={avatarId} alt={name} className="agent-activity-avatar__image" />
    </span>
  );
}
