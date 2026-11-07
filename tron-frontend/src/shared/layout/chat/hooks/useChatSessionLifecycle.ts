import { useEffect, useRef } from "react";
import { useLocation, useNavigate } from "react-router";
import { CHAT_SESSIONS_REMOVED_EVENT, type RemovedChatScope } from "@/shared/lib/shellEvents";
import { useTabStore } from "@/store/tabStore";

function sessionCodeFromPath(path: string) {
  const url = new URL(path, "https://tron.local");
  return url.pathname === "/chats" ? url.searchParams.get("sessionCode") : null;
}

/** Archive/delete completion applies to all tabs, including inactive workspaces. */
export function useChatSessionLifecycle() {
  const navigate = useNavigate();
  const location = useLocation();
  const locationRef = useRef(location);
  locationRef.current = location;

  useEffect(() => {
    const removeSessions = (event: Event) => {
      const detail = (event as CustomEvent<RemovedChatScope & { sessionCodes: string[] }>).detail;
      if (!detail) return;
      const removed = new Set(detail.sessionCodes);
      const removedPath = (path: string) => {
        if (removed.has(sessionCodeFromPath(path) ?? "")) return true;
        const url = new URL(path, "https://tron.local");
        return url.pathname === "/chats" && Boolean(url.searchParams.get("draftCode")) && (
          detail.projectCodes?.includes(url.searchParams.get("projectCode") ?? "")
          || detail.directoryCodes?.includes(url.searchParams.get("directoryCode") ?? "")
        );
      };
      const state = useTabStore.getState();
      const tabIds = state.getAllTabs().filter((tab) => (
        removedPath(tab.path)
        || (tab.activeNav?.startsWith("chat-session:")
          && removed.has(tab.activeNav.slice("chat-session:".length)))
      )).map((tab) => tab.id);
      const current = locationRef.current;
      const removedLocation = removedPath(`${current.pathname}${current.search}`);
      state.removeTabs(tabIds);
      if (removedLocation) {
        const updated = useTabStore.getState();
        const next = updated.tabs.find((tab) => tab.id === updated.activeTabId && !tab.hide);
        // Replace the dead URL too, so refresh cannot reopen the removed chat.
        void navigate(next?.path ?? "/workspace", { replace: true });
      }
    };
    window.addEventListener(CHAT_SESSIONS_REMOVED_EVENT, removeSessions);
    return () => window.removeEventListener(CHAT_SESSIONS_REMOVED_EVENT, removeSessions);
  }, [navigate]);
}
