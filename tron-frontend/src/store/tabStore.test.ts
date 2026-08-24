import { afterEach, describe, expect, it } from "vitest";

import { useTabStore } from "./tabStore";

describe("tab creation suppression", () => {
  afterEach(() => {
    useTabStore.getState().clearAllTabs();
    useTabStore.getState().clearTabCreationSuppression();
  });

  it("keeps the empty workspace route suppressed until navigation resumes", () => {
    const store = useTabStore.getState();

    store.suppressNextTab();
    expect(useTabStore.getState()).toMatchObject({
      suppressNextTabCreation: true,
      tabCreationSuppressionReachedTarget: false,
      tabCreationSuppressionTargetPath: "/apps",
    });

    useTabStore.getState().markTabCreationSuppressionTargetReached();
    expect(useTabStore.getState().tabCreationSuppressionReachedTarget).toBe(true);

    useTabStore.getState().clearTabCreationSuppression();
    expect(useTabStore.getState()).toMatchObject({
      suppressNextTabCreation: false,
      tabCreationSuppressionReachedTarget: false,
      tabCreationSuppressionTargetPath: null,
    });
  });

  it("removes multiple tabs and refreshes a cached tab", () => {
    const firstTabId = useTabStore.getState().addTab({ closable: true, path: "/apps", title: "Apps" });
    const secondTabId = useTabStore.getState().addTab({ closable: true, path: "/tasks", title: "Tasks" });
    const thirdTabId = useTabStore.getState().addTab({ closable: true, path: "/chats", title: "Chats" });

    useTabStore.getState().addToCache(secondTabId, "/tasks", "tasks.index");
    useTabStore.getState().removeTabs([firstTabId, thirdTabId]);

    expect(useTabStore.getState().tabs.map((tab) => tab.id)).toEqual([secondTabId]);
    expect(useTabStore.getState().activeTabId).toBe(secondTabId);

    useTabStore.getState().refreshTab(secondTabId);
    expect(useTabStore.getState().cachedTabsMap.get(secondTabId)?.revision).toBe(1);
  });
});
