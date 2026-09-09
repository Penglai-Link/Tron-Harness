import { afterEach, describe, expect, it } from "vitest";

import { DEFAULT_TAB_ID, useTabStore } from "./tabStore";

describe("tab store", () => {
  afterEach(() => {
    useTabStore.getState().updateRightPanelState(DEFAULT_TAB_ID, {
      open: false,
      size: 520,
    });
    useTabStore.getState().clearAllTabs();
  });

  it("starts with an active hidden default tab", () => {
    const state = useTabStore.getState();

    expect(state.activeTabId).toBe(DEFAULT_TAB_ID);
    expect(state.tabs).toEqual([
      expect.objectContaining({
        id: DEFAULT_TAB_ID,
        hide: true,
        rightPanelOpen: false,
        rightPanelSize: 520,
      }),
    ]);
  });

  it("removes multiple tabs and refreshes a cached tab", () => {
    const firstTabId = useTabStore
      .getState()
      .addTab({ closable: true, path: "/apps", title: "Apps" });
    const secondTabId = useTabStore
      .getState()
      .addTab({ closable: true, path: "/tasks", title: "Tasks" });
    const thirdTabId = useTabStore
      .getState()
      .addTab({ closable: true, path: "/chats", title: "Chats" });

    useTabStore.getState().addToCache(secondTabId, "/tasks", "tasks.index");
    useTabStore.getState().removeTabs([firstTabId, thirdTabId]);

    expect(useTabStore.getState().tabs.map((tab) => tab.id)).toEqual([
      DEFAULT_TAB_ID,
      secondTabId,
    ]);
    expect(useTabStore.getState().activeTabId).toBe(secondTabId);

    useTabStore.getState().refreshTab(secondTabId);
    expect(
      useTabStore.getState().cachedTabsMap.get(secondTabId)?.revision
    ).toBe(1);
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
      ])
    );

    useTabStore
      .getState()
      .updateRightPanelState(firstTabId, { open: true, size: 640 });
    useTabStore
      .getState()
      .updateRightPanelState(secondTabId, { open: false, size: 440 });

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
      ])
    );
  });

  it("activates the default tab after the last ordinary tab is removed", () => {
    const tabId = useTabStore.getState().addTab({
      closable: true,
      path: "/apps",
      title: "Apps",
    });
    useTabStore.getState().addToCache(DEFAULT_TAB_ID, "/default", "default");
    useTabStore.getState().addToCache(tabId, "/apps", "apps.index");

    useTabStore.getState().closeTab(tabId);

    expect(useTabStore.getState().tabs.map((tab) => tab.id)).toEqual([
      DEFAULT_TAB_ID,
    ]);
    expect(useTabStore.getState().activeTabId).toBe(DEFAULT_TAB_ID);
    expect(
      useTabStore.getState().cachedTabsList.map((tab) => tab.tabId)
    ).toEqual([DEFAULT_TAB_ID]);
  });

  it("keeps the default tab and its panel state when clearing ordinary tabs", () => {
    useTabStore.getState().updateRightPanelState(DEFAULT_TAB_ID, {
      open: true,
      size: 640,
    });
    useTabStore
      .getState()
      .addTab({ closable: true, path: "/apps", title: "Apps" });

    useTabStore.getState().clearAllTabs();

    expect(useTabStore.getState().activeTabId).toBe(DEFAULT_TAB_ID);
    expect(useTabStore.getState().tabs).toEqual([
      expect.objectContaining({
        id: DEFAULT_TAB_ID,
        rightPanelOpen: true,
        rightPanelSize: 640,
      }),
    ]);
    expect(useTabStore.getState().cachedTabsList).toEqual([]);
  });

  it("resets the default tab and cache state for a new session", () => {
    useTabStore.getState().updateRightPanelState(DEFAULT_TAB_ID, {
      open: true,
      size: 640,
    });
    const tabId = useTabStore.getState().addTab({
      closable: true,
      path: "/apps",
      title: "Apps",
    });
    useTabStore.getState().addToCache(tabId, "/apps", "apps.index");
    const previousResetVersion = useTabStore.getState().resetVersion;

    useTabStore.getState().resetAllTabs();

    expect(useTabStore.getState().tabs).toEqual([
      expect.objectContaining({
        id: DEFAULT_TAB_ID,
        rightPanelOpen: false,
        rightPanelSize: 520,
      }),
    ]);
    expect(useTabStore.getState().activeTabId).toBe(DEFAULT_TAB_ID);
    expect(useTabStore.getState().cachedTabsList).toEqual([]);
    expect(useTabStore.getState().resetVersion).toBe(previousResetVersion + 1);
  });
});
