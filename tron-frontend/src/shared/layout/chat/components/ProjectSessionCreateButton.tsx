import { useState } from "react";
import { SquarePen } from "lucide-react";
import { useNavigate } from "react-router";
import { createChatSession } from "@/domain/chat/hooks";
import type { ChatSession } from "@/domain/chat/model";
import type { ProjectRecord } from "@/domain/project";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";

export function ProjectSessionCreateButton({
  project,
  sessions,
}: {
  project: ProjectRecord;
  sessions: ChatSession[];
}) {
  const navigate = useNavigate();
  const toast = useToast();
  const [creating, setCreating] = useState(false);

  async function createSession() {
    if (creating) return;
    const emptySession = sessions.find((session) => !session.lastRunCode);
    if (emptySession) {
      navigate(`/chats?sessionCode=${encodeURIComponent(emptySession.sessionCode)}`);
      return;
    }
    setCreating(true);
    try {
      const session = await createChatSession({
        workspaceCode: project.workspaceCode,
        projectCode: project.projectCode,
        title: "New chat",
        agentCode: "main-agent",
      });
      window.dispatchEvent(new Event("tron:chat-session-created"));
      navigate(`/chats?sessionCode=${encodeURIComponent(session.sessionCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "创建会话失败");
    } finally {
      setCreating(false);
    }
  }

  return (
    <Button
      className="shrink-0"
      disabled={creating}
      type="button"
      variant="link"
      onClick={(event) => {
        event.stopPropagation();
        void createSession();
      }}
    >
      <SquarePen size={14} />
    </Button>
  );
}
