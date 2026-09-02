import { afterEach, describe, expect, it } from "vitest";

import { useTabStore } from "./tabStore";

describe("tab store", () => {
  afterEach(() => {
    useTabStore.getState().clearAllTabs();
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
