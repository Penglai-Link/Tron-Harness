import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { useTabStore } from "@/store/tabStore";

/** A draft is a local tab. Only the first submission creates a server session. */
export function useCreateChatTab(workspaceCode: string, projectCode?: string) {
  const navigate = useNavigate();
  const { t } = useTranslation("shell");

  function create() {
    if (!workspaceCode) return;
    const draftCode = crypto.randomUUID();
    const params = new URLSearchParams({ draftCode });
    if (projectCode) params.set("projectCode", projectCode);
    const path = `/chats?${params}`;
    const tabId = useTabStore.getState().addTab({
      title: t("chatSidebar.newChat"),
      titleTranslationKey: "shell:chatSidebar.newChat",
      path,
      closable: true,
      activeNav: `chat-draft:${draftCode}`,
    });
    useTabStore.getState().setActiveTab(tabId);
    navigate(path);
  }
  return { create, creating: false };
}
