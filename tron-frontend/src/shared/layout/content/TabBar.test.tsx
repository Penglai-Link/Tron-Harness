// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useTabStore } from "@/store/tabStore";

import { TabBar } from "./TabBar";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: vi.fn() },
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

  it("keeps the current location when closing the last visible tab", () => {
    useTabStore.getState().addTab({ closable: true, path: "/apps", title: "Apps" });

    function LocationProbe() {
      const location = useLocation();
      return <output data-testid="location">{location.pathname}</output>;
    }

    render(
      <MemoryRouter initialEntries={["/apps"]}>
        <LocationProbe />
        <TabBar />
      </MemoryRouter>,
    );

    const tab = screen.getByRole("tab", { name: "Apps" });
    const closeButton = tab.querySelector("button");
    expect(closeButton).not.toBeNull();
    fireEvent.click(closeButton!);

    expect(useTabStore.getState().tabs.filter((item) => !item.hide)).toEqual([]);
    expect(screen.getByTestId("location").textContent).toBe("/apps");
  });
});
