// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { Link, MemoryRouter, useLocation } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import { DEFAULT_TAB_ID, useTabStore } from "@/store/tabStore";
import AppRoutePage from "./AppRoutePage";

vi.mock("@/domain/auth", () => ({
  DEFAULT_AUTH_REDIRECT: "/workspace",
  AuthUserMenuPanel: () => null,
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock("@/shared", () => ({
  TronShell: ({ children }: { children: ReactNode }) => children,
  SettingsShell: ({ children }: { children: ReactNode }) => children,
  TabPanelRegistryProvider: ({ children }: { children: ReactNode }) => children,
}));
vi.mock("./ConsoleFallbackRoutePage", () => ({
  consoleNavigationItems: [],
  resolveActiveItem: () => undefined,
}));
vi.mock("./ConsoleFallbackPage", () => ({ default: () => null }));
vi.mock("@/router/routes", () => ({
  appRoutes: [{ route: "/chats", pageId: "chats.index", title: "Chats" }],
}));
vi.mock("@/router/pageRegistry", () => ({
  routePages: { "chats.index": () => {
    const location = useLocation();
    return <main data-session={location.search}>Session content</main>;
  } },
}));

const sessionPath = "/chats?sessionCode=session-one";

function SessionControls() {
  const location = useLocation();
  return <>
    <output data-testid="location">{location.pathname}{location.search}</output>
    <Link to={sessionPath} onClick={() => useTabStore.getState().addTab({
      title: "Session one", path: sessionPath, closable: true,
      activeNav: "chat-session:session-one",
    })}>Open session</Link>
    <button onClick={() => {
      const state = useTabStore.getState();
      state.closeTab(state.activeTabId!);
    }}>Close session</button>
  </>;
}

afterEach(() => {
  cleanup();
  useTabStore.getState().resetAllTabs();
});

it("restores content when reopening the last closed session at the same URL", async () => {
  render(<MemoryRouter initialEntries={[sessionPath]}>
    <SessionControls />
    <AppRoutePage />
  </MemoryRouter>);

  for (let attempt = 0; attempt < 2; attempt += 1) {
    await screen.findByText("Session content");
    expect(useTabStore.getState().tabs.filter((tab) => !tab.hide)).toHaveLength(1);
    fireEvent.click(screen.getByText("Close session"));
    await waitFor(() => expect(screen.queryByText("Session content")).toBeNull());
    expect(useTabStore.getState().activeTabId).toBe(DEFAULT_TAB_ID);
    expect(useTabStore.getState().cachedTabsList).toHaveLength(0);
    expect(screen.getByTestId("location").textContent).toBe(sessionPath);
    fireEvent.click(screen.getByText("Open session"));
    await screen.findByText("Session content");
    expect(useTabStore.getState().cachedTabsList).toHaveLength(1);
  }
});

it("never injects another session location into a cached chat while switching", async () => {
  const secondPath = "/chats?sessionCode=session-two";
  render(<MemoryRouter initialEntries={[sessionPath]}>
    <Link to={secondPath}>Second session</Link>
    <Link to={sessionPath}>First session</Link>
    <AppRoutePage />
  </MemoryRouter>);
  await screen.findByText("Session content");
  fireEvent.click(screen.getByText("Second session"));
  await waitFor(() => expect(screen.getAllByText("Session content")).toHaveLength(2));
  const contents = screen.getAllByText("Session content");
  expect(contents.map((node) => node.getAttribute("data-session"))).toEqual([
    "?sessionCode=session-one", "?sessionCode=session-two",
  ]);
  fireEvent.click(screen.getByText("First session"));
  await waitFor(() => expect(contents[0].closest("[hidden]")).toBeNull());
  expect(contents[1].getAttribute("data-session")).toBe("?sessionCode=session-two");
});
