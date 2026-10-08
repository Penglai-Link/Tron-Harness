// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import { useTabStore } from "@/store/tabStore";
import { SettingsShell } from "./SettingsShell";

vi.mock("react-i18next", async (importOriginal) => ({
  ...await importOriginal<typeof import("react-i18next")>(),
  useTranslation: () => ({ t: (key: string) => key }),
}));

function Location() {
  const location = useLocation();
  return <output data-testid="location">{location.pathname}{location.search}</output>;
}

afterEach(() => {
  cleanup();
  useTabStore.getState().resetAllTabs();
  window.sessionStorage.removeItem("tron.settings.returnTo");
});

it.each([
  "/chats?draftCode=closed-draft",
  "/chats?sessionCode=closed-session",
  "/agents/internal",
])("returns to the empty workspace after the last tab at %s was closed", (from) => {
  const tab = useTabStore.getState().addTab({ title: "Closed", path: from, closable: true });
  useTabStore.getState().closeTab(tab);
  window.sessionStorage.setItem("tron.settings.returnTo", from);
  render(<MemoryRouter initialEntries={[{ pathname: "/settings/general", state: { from } }]}>
    <SettingsShell activeItem="General" items={[]}><Location /></SettingsShell>
  </MemoryRouter>);
  const back = screen.getByRole("link", { name: "shell:settings.back" });
  expect(back.getAttribute("href")).toBe("/workspace");
  fireEvent.click(back);
  expect(screen.getByTestId("location").textContent).toBe("/workspace");
  expect(useTabStore.getState().tabs.filter((item) => !item.hide)).toHaveLength(0);
  expect(useTabStore.getState().cachedTabsList).toHaveLength(0);
});

it("ignores a stale saved return address when settings is opened directly without tabs", () => {
  window.sessionStorage.setItem("tron.settings.returnTo", "/chats?draftCode=obsolete");
  render(<MemoryRouter initialEntries={["/settings/agents"]}>
    <SettingsShell activeItem="Agents" items={[]}><Location /></SettingsShell>
  </MemoryRouter>);
  expect(screen.getByRole("link", { name: "shell:settings.back" }).getAttribute("href")).toBe("/workspace");
});

it("preserves the return address of an open tab", () => {
  const from = "/chats?draftCode=open-draft";
  useTabStore.getState().addTab({ title: "Draft", path: from, closable: true });
  render(<MemoryRouter initialEntries={[{ pathname: "/settings/agents", state: { from } }]}>
    <SettingsShell activeItem="Agents" items={[]}><Location /></SettingsShell>
  </MemoryRouter>);
  expect(screen.getByRole("link", { name: "shell:settings.back" }).getAttribute("href")).toBe(from);
});
