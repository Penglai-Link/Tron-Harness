import { SquarePen } from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router";
import { createChatSession } from "@/domain/chat/hooks";
import type { ChatSession } from "@/domain/chat/model";
import type { ProjectRecord } from "@/domain/project";
import IconButton from "./IconButton";

export function Create({
  project,
  sessions,
  workspaceCode,
}: {
  project?: ProjectRecord;
  sessions: ChatSession[];
  workspaceCode: string;
}) {
  const navigate = useNavigate();
  const [creating, setCreating] = useState(false);
  async function create() {
    if (!project || creating) return;
    const emptySession = sessions.find((session) => !session.lastRunCode);
    if (emptySession) {
      navigate(
        `/chats?sessionCode=${encodeURIComponent(emptySession.sessionCode)}`,
      );
      return;
    }
    setCreating(true);
    try {
      const session = await createChatSession({
        workspaceCode,
        projectCode: project.projectCode,
        title: "New chat",
        agentCode: "main-agent",
      });
      window.dispatchEvent(new Event("tron:chat-session-created"));
      navigate(`/chats?sessionCode=${encodeURIComponent(session.sessionCode)}`);
    } finally {
      setCreating(false);
    }
  }
  return (
    <IconButton
      disabled={!project || creating}
      onClick={() => void create()}
      icon={SquarePen}
      className="w-full justify-start"
    >
      新会话
    </IconButton>
  );
}
