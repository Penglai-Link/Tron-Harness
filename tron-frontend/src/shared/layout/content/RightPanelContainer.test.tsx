// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useEffect } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { EditableTab } from "@/components/editable-tabs";
import { useTabStore } from "@/store/tabStore";

import RightPanelContainer from "./RightPanelContainer";
import { TabPanelRegistryProvider, useRightPanelTabs } from "./TabPanelRegistry";

vi.mock("@/components/page-view", () => ({
  PageView: ({ url }: { url: string }) => (
    <input aria-label={url} defaultValue="" />
  ),
}));

function TabOpener({ onReady }: { onReady: (open: (tab: EditableTab) => void) => void }) {
  const { openTab } = useRightPanelTabs();

  useEffect(() => onReady(openTab), [onReady, openTab]);
  return null;
}

describe("RightPanelContainer", () => {
  afterEach(() => {
    cleanup();
    useTabStore.getState().clearAllTabs();
  });

  it("opens a separate dynamic tab for each URL in the current main content tab", () => {
    const tabId = useTabStore.getState().addTab({
      closable: true,
      path: "/chats",
      title: "Chats",
    });
    useTabStore.getState().addToCache(tabId, "/chats", "chats.index");
    let openTab: ((tab: EditableTab) => void) | undefined;

    render(
      <TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId={tabId}>
          <TabOpener onReady={(open) => { openTab = open; }} />
        </TabPanelRegistryProvider>
        <RightPanelContainer />
      </TabPanelRegistryProvider>,
    );

    act(() => openTab?.({
      children: <div>https://www.tencent.com</div>,
      key: "markdown-page:https://www.tencent.com",
      label: "https://www.tencent.com",
    }));
    act(() => openTab?.({
      children: <div>https://www.baidu.com</div>,
      key: "markdown-page:https://www.baidu.com",
      label: "https://www.baidu.com",
    }));

    expect(screen.getByRole("tab", { name: "https://www.tencent.com" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "https://www.baidu.com" })).toBeTruthy();
  });

  it("preserves panel state independently for each main content tab", () => {
    const firstTabId = useTabStore.getState().addTab({
      closable: true,
      path: "/chats/first",
      title: "First",
    });
    const secondTabId = useTabStore.getState().addTab({
      closable: true,
      path: "/chats/second",
      title: "Second",
    });
    useTabStore.getState().addToCache(firstTabId, "/chats/first", "chats.index");
    useTabStore.getState().addToCache(secondTabId, "/chats/second", "chats.index");
    let openFirstTab: ((tab: EditableTab) => void) | undefined;
    let openSecondTab: ((tab: EditableTab) => void) | undefined;

    render(
      <TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId={firstTabId}>
          <TabOpener onReady={(open) => { openFirstTab = open; }} />
        </TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId={secondTabId}>
          <TabOpener onReady={(open) => { openSecondTab = open; }} />
        </TabPanelRegistryProvider>
        <RightPanelContainer />
      </TabPanelRegistryProvider>,
    );

    act(() => {
      openFirstTab?.({
        children: null,
        key: "first-page",
        label: "First page",
        stableId: "first-page",
        url: "https://first.example.com",
      });
      openSecondTab?.({
        children: null,
        key: "second-page",
        label: "Second page",
        stableId: "second-page",
        url: "https://second.example.com",
      });
      useTabStore.getState().setActiveTab(firstTabId);
    });

    const firstInput = screen.getByRole("textbox", { name: "https://first.example.com" });
    fireEvent.change(firstInput, { target: { value: "first draft" } });

    act(() => useTabStore.getState().setActiveTab(secondTabId));
    const secondInput = screen.getByRole("textbox", { name: "https://second.example.com" });
    fireEvent.change(secondInput, { target: { value: "second draft" } });

    act(() => useTabStore.getState().setActiveTab(firstTabId));
    expect((screen.getByRole("textbox", { name: "https://first.example.com" }) as HTMLInputElement).value)
      .toBe("first draft");

    act(() => useTabStore.getState().setActiveTab(secondTabId));
    expect((screen.getByRole("textbox", { name: "https://second.example.com" }) as HTMLInputElement).value)
      .toBe("second draft");
  });

  it("keeps an iframe tab mounted while switching between panel tabs", () => {
    const tabId = useTabStore.getState().addTab({
      closable: true,
      path: "/chats",
      title: "Chats",
    });
    useTabStore.getState().addToCache(tabId, "/chats", "chats.index");
    let openTab: ((tab: EditableTab) => void) | undefined;

    render(
      <TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId={tabId}>
          <TabOpener onReady={(open) => { openTab = open; }} />
        </TabPanelRegistryProvider>
        <RightPanelContainer />
      </TabPanelRegistryProvider>,
    );

    act(() => {
      openTab?.({
        children: null,
        key: "first-page",
        label: "First page",
        stableId: "first-page",
        url: "https://first.example.com",
      });
      openTab?.({
        children: null,
        key: "second-page",
        label: "Second page",
        stableId: "second-page",
        url: "https://second.example.com",
      });
    });

    fireEvent.click(screen.getByRole("tab", { name: "First page" }));
    const firstInput = screen.getByRole("textbox", { name: "https://first.example.com" });
    fireEvent.change(firstInput, { target: { value: "first draft" } });

    fireEvent.click(screen.getByRole("tab", { name: "Second page" }));
    fireEvent.click(screen.getByRole("tab", { name: "First page" }));

    expect((screen.getByRole("textbox", { name: "https://first.example.com" }) as HTMLInputElement).value)
      .toBe("first draft");
  });
});
