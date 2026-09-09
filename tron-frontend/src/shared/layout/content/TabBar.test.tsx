// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useTabStore } from "@/store/tabStore";

import { TabBar } from "./TabBar";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => ({
      "tabs.closeLeft": "Close left tabs",
      "tabs.closeOthers": "Close other tabs",
      "tabs.closeRight": "Close right tabs",
      "tabs.refresh": "Refresh",
    })[key] ?? key,
  }),
}));

describe("TabBar", () => {
  beforeEach(() => {
    useTabStore.getState().clearAllTabs();
  });

  afterEach(() => {
    cleanup();
    useTabStore.getState().clearAllTabs();
  });

  it("closes other tabs from a tab context menu", async () => {
    useTabStore.getState().addTab({ closable: true, path: "/apps", title: "Apps" });
    const tasksTabId = useTabStore.getState().addTab({ closable: true, path: "/tasks", title: "Tasks" });
    useTabStore.getState().addTab({ closable: true, path: "/chats", title: "Chats" });

    render(
      <MemoryRouter initialEntries={["/tasks"]}>
        <TabBar />
      </MemoryRouter>,
    );

    fireEvent.contextMenu(screen.getByRole("tab", { name: "Tasks" }));
    fireEvent.click(await screen.findByText("Close other tabs"));

    await waitFor(() => expect(useTabStore.getState().tabs.map((tab) => tab.id)).toEqual([tasksTabId]));
  });
});
