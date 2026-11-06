import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { useTabStore } from "@/store/tabStore";

/** A draft is a local tab. Only the first submission creates a server session. */
export function useCreateChatTab(workspaceCode: string, projectCode?: string, directoryCode?: string) {
  const navigate = useNavigate();
  const { t } = useTranslation("shell");

  function create(targetDirectoryCode = directoryCode) {
    if (!workspaceCode) return;
    const tabs = useTabStore.getState();
    const existingDraft = tabs.tabs.find((tab) => {
      if (!tab.activeNav?.startsWith("chat-draft:")) return false;
      const [pathname, search = ""] = tab.path.split("?");
      if (pathname !== "/chats") return false;
      const params = new URLSearchParams(search);
      return Boolean(params.get("draftCode"))
        && params.get("workspaceCode") === workspaceCode
        && (params.get("projectCode") ?? undefined) === projectCode
        && (params.get("directoryCode") ?? undefined) === targetDirectoryCode;
    });
    if (existingDraft) {
      tabs.setActiveTab(existingDraft.id);
      navigate(existingDraft.path);
      return;
    }
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
    });
    useTabStore.getState().setActiveTab(tabId);
    navigate(path);
  }
  return { create: () => create(), createForDirectory: (code: string) => create(code), creating: false };
}
