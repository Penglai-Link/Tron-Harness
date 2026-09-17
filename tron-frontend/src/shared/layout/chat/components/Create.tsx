import { SquarePen } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { createChatSession } from "@/domain/chat/hooks";
import type { ProjectRecord } from "@/domain/project";
import { useTabStore } from "@/store/tabStore";
import { useToast } from "@/components/ui/toast";
import IconButton from "./IconButton";

export function Create({
  project,
  workspaceCode,
}: {
  project?: ProjectRecord;
  workspaceCode: string;
}) {
  const navigate = useNavigate();
  const { t } = useTranslation("shell");
  const toast = useToast();
  const [creating, setCreating] = useState(false);
  async function create() {
    if (!workspaceCode || creating) return;
    setCreating(true);
    try {
      const session = await createChatSession({
        workspaceCode,
        projectCode: project?.projectCode,
        title: "New chat",
        agentCode: "main-agent",
      });
      window.dispatchEvent(new Event("tron:chat-session-created"));
      const path = `/chats?sessionCode=${encodeURIComponent(session.sessionCode)}`;
      const tabId = useTabStore.getState().addTab({
        title: session.title || "New chat",
        path,
        closable: true,
        activeNav: `chat-session:${session.sessionCode}`,
      });
      useTabStore.getState().updateTab(tabId, { path });
      navigate(path);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("shell:chatSidebar.session.createFailed"));
    } finally {
      setCreating(false);
    }
  }
  return (
    <IconButton
      disabled={!workspaceCode || creating}
      onClick={() => void create()}
      icon={SquarePen}
      className="w-full justify-start"
    >
      {t("shell:chatSidebar.newChat")}
    </IconButton>
  );
}
