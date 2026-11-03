import { useCallback } from "react";
import { DEFAULT_TAB_ID, useTabStore } from "@/store/tabStore";

export function useOpenRightSidebar() {
  const activeTabId = useTabStore((state) => state.activeTabId ?? DEFAULT_TAB_ID);
  const updateRightPanelState = useTabStore((state) => state.updateRightPanelState);

  return useCallback(() => {
    if (activeTabId) {
      updateRightPanelState(activeTabId, { open: true });
    }
  }, [activeTabId, updateRightPanelState]);
}
