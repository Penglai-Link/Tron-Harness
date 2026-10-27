// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useTabStore } from "@/store/tabStore";
import { useCreateChatTab } from "./useCreateChatTab";

vi.mock("react-i18next", async (importOriginal) => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: () => "新会话" }) }));
afterEach(() => { cleanup(); useTabStore.getState().resetAllTabs(); vi.unstubAllGlobals(); });

describe("new chat tabs", () => {
  it("reuses an existing draft even after another tab becomes active", () => {
    const fetch = vi.fn();
    vi.stubGlobal("fetch", fetch);
    const created = vi.fn();
    window.addEventListener("tron:chat-session-created", created);
    const { result } = renderHook(() => ({ ...useCreateChatTab("workspace", "project"), location: useLocation() }), {
      wrapper: ({ children }) => <MemoryRouter>{children}</MemoryRouter>,
    });
    act(() => { result.current.create(); });
    const first = new URLSearchParams(result.current.location.search);
    expect(first.get("draftCode")).toBeTruthy();
    expect(first.get("projectCode")).toBe("project");
    expect(first.has("sessionCode")).toBe(false);
    const draft = useTabStore.getState().tabs.find((tab) => tab.activeNav === `chat-draft:${first.get("draftCode")}`)!;
    act(() => { useTabStore.getState().addTab({ title: "Other", path: "/other", closable: true }); });
    act(() => { result.current.create(); });
    expect(new URLSearchParams(result.current.location.search).get("draftCode")).toBe(first.get("draftCode"));
    expect(useTabStore.getState().activeTabId).toBe(draft.id);
    expect(useTabStore.getState().tabs.filter((tab) => tab.activeNav?.startsWith("chat-draft:"))).toHaveLength(1);
    expect(fetch).not.toHaveBeenCalled();
    expect(created).not.toHaveBeenCalled();
    window.removeEventListener("tron:chat-session-created", created);
  });

  it("creates a fresh draft after the existing draft becomes a session", () => {
    const { result } = renderHook(() => ({ ...useCreateChatTab("workspace"), location: useLocation() }), {
      wrapper: ({ children }) => <MemoryRouter>{children}</MemoryRouter>,
    });
    act(() => { result.current.create(); });
    const first = new URLSearchParams(result.current.location.search).get("draftCode");
    const id = useTabStore.getState().activeTabId!;
    act(() => { useTabStore.getState().updateTab(id, { path: "/chats?sessionCode=started", activeNav: "chat-session:started" }); });
    act(() => { result.current.create(); });
    expect(new URLSearchParams(result.current.location.search).get("draftCode")).not.toBe(first);
    expect(useTabStore.getState().tabs.filter((tab) => tab.activeNav?.startsWith("chat-draft:"))).toHaveLength(1);
  });

  it("keeps drafts for different projects separate", () => {
    const { result, rerender } = renderHook(({ project }) => ({ ...useCreateChatTab("workspace", project), location: useLocation() }), {
      initialProps: { project: "project-a" },
      wrapper: ({ children }) => <MemoryRouter>{children}</MemoryRouter>,
    });
    act(() => { result.current.create(); });
    const first = new URLSearchParams(result.current.location.search).get("draftCode");
    rerender({ project: "project-b" });
    act(() => { result.current.create(); });
    expect(new URLSearchParams(result.current.location.search).get("draftCode")).not.toBe(first);
    expect(new URLSearchParams(result.current.location.search).get("projectCode")).toBe("project-b");
    rerender({ project: "project-a" });
    act(() => { result.current.create(); });
    expect(new URLSearchParams(result.current.location.search).get("draftCode")).toBe(first);
  });

  it("reuses a project draft from the general new chat action", () => {
    const { result, rerender } = renderHook(({ project }: { project?: string }) => ({ ...useCreateChatTab("workspace", project), location: useLocation() }), {
      initialProps: { project: "project-a" as string | undefined },
      wrapper: ({ children }) => <MemoryRouter>{children}</MemoryRouter>,
    });
    act(() => { result.current.create(); });
    const firstPath = result.current.location.search;
    rerender({ project: undefined });
    act(() => { result.current.create(); });
    expect(result.current.location.search).toBe(firstPath);
    expect(useTabStore.getState().tabs.filter((tab) => tab.activeNav?.startsWith("chat-draft:"))).toHaveLength(1);
  });
});
