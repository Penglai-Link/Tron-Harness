import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { useTabStore } from "@/store/tabStore";

/** A draft is a local tab. Only the first submission creates a server session. */
export function useCreateChatTab(workspaceCode: string, projectCode?: string, directoryCode?: string) {
  const navigate = useNavigate();
  const { t } = useTranslation("shell");

  function create(targetDirectoryCode = directoryCode, initialChatMessage?: string) {
    if (!workspaceCode) return;
    const draftCode = crypto.randomUUID();
    const params = new URLSearchParams({ draftCode, workspaceCode });
    if (projectCode) params.set("projectCode", projectCode);
    if (targetDirectoryCode) params.set("directoryCode", targetDirectoryCode);
    const path = `/chats?${params}`;
    const tabId = useTabStore.getState().addTab({
      title: t("chatSidebar.newChat"),
      titleTranslationKey: "shell:chatSidebar.newChat",
      path,
      closable: true,
      activeNav: `chat-draft:${draftCode}`,
      initialChatMessage,
    });
    useTabStore.getState().setActiveTab(tabId);
    navigate(path);
  }
  return {
    create: () => create(),
    createForDirectory: (code: string) => create(code),
    createWithMessage: (message: string) => create(undefined, message),
    creating: false,
  };
}
