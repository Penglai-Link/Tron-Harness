import { useRef, useState } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { createChatSession } from "@/domain/chat/hooks";
import { useToast } from "@/components/ui/toast";
import { useTabStore } from "@/store/tabStore";

export function useCreateChatTab(workspaceCode: string, projectCode?: string) {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation("shell");
  const pending = useRef(false);
  const [creating, setCreating] = useState(false);

  async function create() {
    if (!workspaceCode || pending.current) return;
    pending.current = true;
    setCreating(true);
    try {
      const session = await createChatSession({ workspaceCode, projectCode, title: "New chat", agentCode: "main-agent" });
      window.dispatchEvent(new CustomEvent("tron:chat-session-created", { detail: session }));
      const path = `/chats?sessionCode=${encodeURIComponent(session.sessionCode)}`;
      const tabId = useTabStore.getState().addTab({
        title: session.title || "New chat", path, closable: true,
        activeNav: `chat-session:${session.sessionCode}`,
      });
      useTabStore.getState().setActiveTab(tabId);
      navigate(path);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("chatSidebar.session.createFailed"));
    } finally {
      pending.current = false;
      setCreating(false);
    }
  }
  return { create, creating };
}
