import { useEffect, useRef } from "react";
import { useLocation, useNavigate } from "react-router";
import { CHAT_SESSIONS_REMOVED_EVENT } from "@/shared/lib/shellEvents";
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
      const sessionCodes = (event as CustomEvent<{ sessionCodes: string[] }>).detail?.sessionCodes;
      if (!sessionCodes?.length) return;
      const removed = new Set(sessionCodes);
      const state = useTabStore.getState();
      const tabIds = state.getAllTabs().filter((tab) => (
        removed.has(sessionCodeFromPath(tab.path) ?? "")
        || (tab.activeNav?.startsWith("chat-session:")
          && removed.has(tab.activeNav.slice("chat-session:".length)))
      )).map((tab) => tab.id);
      const current = locationRef.current;
      const removedLocation = removed.has(sessionCodeFromPath(`${current.pathname}${current.search}`) ?? "");
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
