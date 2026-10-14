// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import { i18n } from "@/i18n";
import { AgentChatWindow } from "./AgentChatWindow";

const feedback = vi.hoisted(() => ({ error: vi.fn(), info: vi.fn() }));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: vi.fn().mockResolvedValue({ agentCode: "main-agent", commands: ["goal"], skills: [{ name: "review", description: "Review changes" }] }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => feedback }));

beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, "scrollTo", { configurable: true, value: vi.fn() });
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); vi.clearAllMocks(); });

describe("AgentChatWindow feedback", () => {
  it.each([
    ["en-US", "Select a workspace before starting a chat", "Chat send failed"],
    ["zh-CN", "请先选择工作区", "发送失败"],
  ])("reports rejected sends through one localized toast in %s", async (locale, message, title) => {
    await i18n.changeLanguage(locale);
    const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
      const headers = new Headers(init?.headers);
      expect(headers.get("X-Workspace-Code")).toBe("WORKSPACE");
      expect(headers.get("Accept-Language")).toBe(locale);
      if (url.endsWith("/stream")) return new Response(JSON.stringify({ code: "CHAT_WORKSPACE_REQUIRED", message: "workspaceCode is required", data: {} }), { status: 400 });
      const data = url.endsWith("/a2ui-state") ? { sessionId: "SESSION", revision: 0, surfaces: [], pendingInteractions: [] }
        : url.endsWith("/history/query") ? { sessionId: "SESSION", version: "1", nextCursor: null, runs: [], records: { steps: [], messages: [], toolInvocations: [], interactions: [] }, pendingInteractions: [], activeRuns: [] }
        : url.includes("get-active-run") ? { sessionCode: "SESSION", activeRun: null }
        : url.endsWith("/events") ? { events: [] } : [];
      return new Response(JSON.stringify({ code: "SUCCESS", message: "ok", data }));
    });
    vi.stubGlobal("fetch", fetchMock);
    const adapter = createInternalChatAdapter({ workspaceCode: "WORKSPACE", getHeaders: () => ({ "Accept-Language": document.documentElement.lang }) });
    render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><AgentChatWindow adapter={adapter} config={{ workspaceCode: "WORKSPACE", floating: false }} initialSession={{ id: 1, sessionCode: "SESSION", workspaceCode: "WORKSPACE", title: "Chat", status: "ACTIVE", createdAt: "2026-10-07T00:00:00Z", updatedAt: "2026-10-07T00:00:00Z" }} /></QueryClientProvider>);
    await waitFor(() => expect(fetchMock).toHaveBeenCalled());
    const input = screen.getByRole("textbox");
    fireEvent.change(input, { target: { value: "hello" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(feedback.error).toHaveBeenCalledExactlyOnceWith(message, { title }));
    expect(screen.queryByText("workspaceCode is required")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getAllByRole("article")).toHaveLength(1);
  });
});
