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

  it("keeps right panel state independent for each tab", () => {
    const firstTabId = useTabStore.getState().addTab({
      closable: true,
      path: "/apps",
      title: "Apps",
    });
    const secondTabId = useTabStore.getState().addTab({
      closable: true,
      path: "/tasks",
      title: "Tasks",
    });

    expect(useTabStore.getState().tabs).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: firstTabId,
          rightPanelOpen: false,
          rightPanelSize: 520,
        }),
        expect.objectContaining({
          id: secondTabId,
          rightPanelOpen: false,
          rightPanelSize: 520,
        }),
      ]),
    );

    useTabStore.getState().updateRightPanelState(firstTabId, { open: true, size: 640 });
    useTabStore.getState().updateRightPanelState(secondTabId, { open: false, size: 440 });

    expect(useTabStore.getState().tabs).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: firstTabId,
          rightPanelOpen: true,
          rightPanelSize: 640,
        }),
        expect.objectContaining({
          id: secondTabId,
          rightPanelOpen: false,
          rightPanelSize: 440,
        }),
      ]),
    );
  });
});
