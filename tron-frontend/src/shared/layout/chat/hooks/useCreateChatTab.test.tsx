// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useTabStore } from "@/store/tabStore";
import { useCreateChatTab } from "./useCreateChatTab";

vi.mock("react-i18next", async (importOriginal) => ({ ...await importOriginal<typeof import("react-i18next")>(), useTranslation: () => ({ t: () => "新会话" }) }));
afterEach(() => { cleanup(); useTabStore.getState().resetAllTabs(); vi.unstubAllGlobals(); });

describe("new chat tabs", () => {
  it("opens separate local drafts without creating sessions or publishing sidebar events", () => {
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
    act(() => { result.current.create(); });
    expect(new URLSearchParams(result.current.location.search).get("draftCode")).not.toBe(first.get("draftCode"));
    expect(useTabStore.getState().tabs.filter((tab) => tab.activeNav?.startsWith("chat-draft:"))).toHaveLength(2);
    expect(fetch).not.toHaveBeenCalled();
    expect(created).not.toHaveBeenCalled();
    window.removeEventListener("tron:chat-session-created", created);
  });
});
