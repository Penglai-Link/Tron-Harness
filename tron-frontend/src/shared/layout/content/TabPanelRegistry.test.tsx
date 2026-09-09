// @vitest-environment jsdom
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { useEffect } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { EditableTab } from "@/components/editable-tabs";

import {
  useTabPanelConfig,
  TabPanelRegistryProvider,
  useRightPanelTabs,
  useTabPanelRegistry,
} from "./TabPanelRegistry";
import { DEFAULT_TAB_ID, useTabStore } from "@/store/tabStore";

const pageTab: EditableTab = {
  key: "markdown-page:https://example.com",
  label: "https://example.com",
  children: null,
};

function TabOpener({
  onReady,
}: {
  onReady: (open: (tab: EditableTab) => void) => void;
}) {
  const { openTab } = useRightPanelTabs();
  onReady(openTab);
  return null;
}

function TabHandler({
  onOpen,
  tabId,
}: {
  onOpen: (tab: EditableTab) => void;
  tabId: string;
}) {
  const { getRightPanelTabs } = useTabPanelRegistry();
  const tabs = getRightPanelTabs(tabId);

  useEffect(() => {
    const tab = tabs[0];
    if (tab) onOpen(tab);
  }, [onOpen, tabs]);
  return null;
}

function PanelRegistrar() {
  useTabPanelConfig({ rightPanel: <div>Page attributes</div> });
  return <TabOpener onReady={() => undefined} />;
}

describe("TabPanelRegistry", () => {
  afterEach(() => {
    cleanup();
  });

  it("delivers a right-panel tab opened before its handler mounts", () => {
    const onOpen = vi.fn();
    let openTab: ((tab: EditableTab) => void) | undefined;
    const { rerender } = render(
      <TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId="chat-a">
          <TabOpener
            onReady={(open) => {
              openTab = open;
            }}
          />
        </TabPanelRegistryProvider>
      </TabPanelRegistryProvider>
    );

    act(() => openTab?.(pageTab));
    expect(onOpen).not.toHaveBeenCalled();

    rerender(
      <TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId="chat-a">
          <TabOpener
            onReady={(open) => {
              openTab = open;
            }}
          />
        </TabPanelRegistryProvider>
        <TabHandler onOpen={onOpen} tabId="chat-a" />
      </TabPanelRegistryProvider>
    );

    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(onOpen).toHaveBeenCalledWith(pageTab);
  });

  it("keeps pending tabs isolated by main content tab", () => {
    const firstHandler = vi.fn();
    const secondHandler = vi.fn();
    let firstOpenTab: ((tab: EditableTab) => void) | undefined;
    let secondOpenTab: ((tab: EditableTab) => void) | undefined;

    const { rerender } = render(
      <TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId="chat-a">
          <TabOpener
            onReady={(open) => {
              firstOpenTab = open;
            }}
          />
        </TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId="chat-b">
          <TabOpener
            onReady={(open) => {
              secondOpenTab = open;
            }}
          />
        </TabPanelRegistryProvider>
      </TabPanelRegistryProvider>
    );

    act(() => {
      firstOpenTab?.(pageTab);
      secondOpenTab?.(pageTab);
    });

    rerender(
      <TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId="chat-a">
          <TabOpener
            onReady={(open) => {
              firstOpenTab = open;
            }}
          />
        </TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId="chat-b">
          <TabOpener
            onReady={(open) => {
              secondOpenTab = open;
            }}
          />
        </TabPanelRegistryProvider>
        <TabHandler onOpen={firstHandler} tabId="chat-a" />
        <TabHandler onOpen={secondHandler} tabId="chat-b" />
      </TabPanelRegistryProvider>
    );

    expect(firstHandler).toHaveBeenCalledWith(pageTab);
    expect(secondHandler).toHaveBeenCalledWith(pageTab);
  });

  it("clears panel configs and dynamic tabs when the tab session resets", async () => {
    let openTab: ((tab: EditableTab) => void) | undefined;
    let registry: ReturnType<typeof useTabPanelRegistry> | undefined;

    function RegistryReader() {
      registry = useTabPanelRegistry();
      return null;
    }

    render(
      <TabPanelRegistryProvider>
        <TabPanelRegistryProvider tabId={DEFAULT_TAB_ID}>
          <PanelRegistrar />
        </TabPanelRegistryProvider>
        <TabOpener
          onReady={(open) => {
            openTab = open;
          }}
        />
        <RegistryReader />
      </TabPanelRegistryProvider>
    );

    await waitFor(() =>
      expect(registry?.get(DEFAULT_TAB_ID)?.rightPanel).toBeDefined()
    );
    act(() => openTab?.(pageTab));
    await waitFor(() =>
      expect(registry?.getRightPanelTabs(DEFAULT_TAB_ID)).toHaveLength(1)
    );

    act(() => useTabStore.getState().resetAllTabs());

    await waitFor(() => {
      expect(registry?.get(DEFAULT_TAB_ID)).toBeUndefined();
      expect(registry?.getRightPanelTabs(DEFAULT_TAB_ID)).toEqual([]);
    });
  });
});
