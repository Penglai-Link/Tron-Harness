import { useState } from "react";
import { SquarePen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { createChatSession } from "@/domain/chat/hooks";
import type { ChatSession } from "@/domain/chat/model";
import type { ProjectRecord } from "@/domain/project";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import IconButton from "./IconButton";

const CreateByProject = ({
  project,
  sessions,
}: {
  project: ProjectRecord;
  sessions: ChatSession[];
}) => {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation("shell");
  const [creating, setCreating] = useState(false);

  async function createSession() {
    if (creating) return;
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
        workspaceCode: project.workspaceCode,
        projectCode: project.projectCode,
        title: "New chat",
        agentCode: "main-agent",
      });
      window.dispatchEvent(new Event("tron:chat-session-created"));
      navigate(`/chats?sessionCode=${encodeURIComponent(session.sessionCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("shell:chatSidebar.session.createFailed"));
    } finally {
      setCreating(false);
    }
  }

  return (
    <IconButton
      className="shrink-0 p-0"
      disabled={creating}
      icon={SquarePen}
      needHover
      onClick={(event) => {
        event.stopPropagation();
        createSession();
      }}
    />
  );
};

export default CreateByProject;
