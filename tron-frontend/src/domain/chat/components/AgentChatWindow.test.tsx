// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import { i18n } from "@/i18n";
import { AgentChatWindow } from "./AgentChatWindow";
import { OPEN_CHAT_CONTEXT_EVENT } from "@/shared/lib/shellEvents";
import { requestApi } from "@/shared/api/httpClient";

const feedback = vi.hoisted(() => ({ error: vi.fn(), info: vi.fn() }));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: vi.fn().mockResolvedValue({ agentCode: "main-agent", commands: ["goal"], skills: [{ name: "review", description: "Review changes" }] }) }));
vi.mock("@/components/ui/toast", () => ({ useToast: () => feedback }));

beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, "scrollTo", { configurable: true, value: vi.fn() });
  vi.mocked(requestApi).mockResolvedValue({ agentCode: "main-agent", commands: ["goal"], skills: [] } as never);
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

it("shows the latest measured model call beside the composer", async () => {
  await i18n.changeLanguage("en-US");
  const audit = JSON.stringify({
    usageSource: "provider", usageConfidence: "actual",
    providerUsage: { inputTokens: 50_000, outputTokens: 200, cachedInputTokens: 25_000 },
    requestEstimate: { estimatedInputTokens: 48_000, contextWindowLimit: 100_000 },
  });
  const event = (eventName: string, occurredAt: string, metadata: Record<string, string>) => ({
    id: eventName, type: eventName, eventName, runCode: "RUN", streamKey: "SESSION",
    occurredAt, outcome: "COMPLETED", metadata,
  });
  vi.stubGlobal("fetch", vi.fn(async (url: string) => {
    const data = url.endsWith("/a2ui-state") ? { sessionId: "SESSION", revision: 0, surfaces: [], pendingInteractions: [] }
      : url.endsWith("/history/query") ? { sessionId: "SESSION", version: "1", nextCursor: null, runs: [], records: { steps: [], messages: [], toolInvocations: [], interactions: [] }, pendingInteractions: [], activeRuns: [] }
      : url.includes("get-active-run") ? { sessionCode: "SESSION", activeRun: null }
      : url.endsWith("/events") ? { events: [
        event("agent.model.requested", "2026-10-19T12:00:00.000Z", { model_attempt_id: "A" }),
        event("agent.model.responded", "2026-10-19T12:00:02.000Z", { model_attempt_id: "A", usage_audit_json: audit }),
      ] } : [];
    return new Response(JSON.stringify({ code: "SUCCESS", data, message: "ok" }));
  }));
  const adapter = createInternalChatAdapter({ workspaceCode: "WORKSPACE" });
  const opened = vi.fn();
  window.addEventListener(OPEN_CHAT_CONTEXT_EVENT, opened);
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <AgentChatWindow adapter={adapter} showHeader={false} config={{ floating: false }}
      initialSession={{ id: 1, sessionCode: "SESSION", workspaceCode: "WORKSPACE", title: "Metrics", status: "ACTIVE", createdAt: "2026-10-19T00:00:00Z", updatedAt: "2026-10-19T00:00:00Z" }} />
  </QueryClientProvider>);
  expect(await screen.findByText("100 tok/s")).toBeTruthy();
  expect(screen.getByText("Cache hit 50%")).toBeTruthy();
  expect(screen.getByText("Context 48%")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Cache hit 50%" }));
  expect(opened).toHaveBeenCalledOnce();
  expect((opened.mock.calls[0]?.[0] as CustomEvent).detail.sessionCode).toBe("SESSION");
  window.removeEventListener(OPEN_CHAT_CONTEXT_EVENT, opened);
});

it("shows all metric controls as zero before the first model call", async () => {
  await i18n.changeLanguage("zh-CN");
  vi.stubGlobal("fetch", vi.fn(async (url: string) => {
    const data = url.endsWith("/a2ui-state") ? { sessionId: "NEW", revision: 0, surfaces: [], pendingInteractions: [] }
      : url.endsWith("/history/query") ? { sessionId: "NEW", version: "1", nextCursor: null, runs: [], records: { steps: [], messages: [], toolInvocations: [], interactions: [] }, pendingInteractions: [], activeRuns: [] }
      : url.includes("get-active-run") ? { sessionCode: "NEW", activeRun: null }
      : url.endsWith("/events") ? { events: [] } : [];
    return new Response(JSON.stringify({ code: "SUCCESS", data, message: "ok" }));
  }));
  const adapter = createInternalChatAdapter({ workspaceCode: "WORKSPACE" });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <AgentChatWindow adapter={adapter} showHeader={false} config={{ floating: false }}
      initialSession={{ id: 1, sessionCode: "NEW", workspaceCode: "WORKSPACE", title: "New", status: "ACTIVE", createdAt: "2026-10-19T00:00:00Z", updatedAt: "2026-10-19T00:00:00Z" }} />
  </QueryClientProvider>);
  expect(screen.getByRole("button", { name: "0 tok/s" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "缓存 0%" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "上下文 0%" })).toBeTruthy();
});

it("restores a legacy Dude context report after closing and reopening the window", async () => {
  await i18n.changeLanguage("zh-CN");
  vi.mocked(requestApi).mockImplementation(async (endpoint) => endpoint.includes("get-context-assembly")
    ? { snapshot: { estimatedInputTokens: 58_208, budgetBreakdown: { contextWindowLimit: 1_310_720 } } } as never
    : { agentCode: "main-agent", commands: [], skills: [] } as never);
  vi.stubGlobal("fetch", vi.fn(async (url: string) => {
    const data = url.endsWith("/a2ui-state") ? { sessionId: "DUDE", revision: 0, surfaces: [], pendingInteractions: [] }
      : url.endsWith("/history/query") ? { sessionId: "DUDE", version: "1", nextCursor: null, runs: [], records: { steps: [], messages: [], toolInvocations: [], interactions: [] }, pendingInteractions: [], activeRuns: [] }
      : url.includes("get-active-run") ? { sessionCode: "DUDE", activeRun: null }
      : url.endsWith("/events") ? { events: [] }
      : url.endsWith("/messages") ? [{ id: "MESSAGE", sessionCode: "DUDE", role: "ASSISTANT", content: "Earlier reply" }]
      : [];
    return new Response(JSON.stringify({ code: "SUCCESS", data, message: "ok" }));
  }));
  const adapter = createInternalChatAdapter({ workspaceCode: null });
  const session = { id: 1, sessionCode: "DUDE", workspaceCode: null, ownerDudeCode: "main-dude", title: "Dude", status: "ACTIVE" as const, createdAt: "2026-10-19T00:00:00Z", updatedAt: "2026-10-19T00:00:00Z" };
  const renderWindow = () => render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <AgentChatWindow adapter={adapter} showHeader={false} config={{ floating: false }} initialSession={session} />
  </QueryClientProvider>);
  const first = renderWindow();
  expect(await screen.findByRole("button", { name: "上下文 4.4%" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "0 tok/s" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "缓存 0%" })).toBeTruthy();
  first.unmount();
  renderWindow();
  expect(await screen.findByRole("button", { name: "上下文 4.4%" })).toBeTruthy();
});

describe("awakening conversation locale", () => {
  it.each([
    ["en-US", "Wake up", "You"],
    ["zh-CN", "醒醒", "你"],
  ])("renders the visible wakeup and running composer in %s", async (locale, wakeup, actor) => {
    await i18n.changeLanguage(locale);
    const run = { id: "RUN", sessionId: "SESSION", parentRunId: null, rootRunId: "RUN", status: "Running", startedAtMs: 1, finishedAtMs: null, error: null };
    vi.stubGlobal("fetch", vi.fn(async (url: string) => {
      const data = url.endsWith("/a2ui-state") ? { sessionId: "SESSION", revision: 0, surfaces: [], pendingInteractions: [] }
        : url.endsWith("/history/query") ? { sessionId: "SESSION", version: "1", nextCursor: null, runs: [run], activeRuns: [run], pendingInteractions: [], records: {
          messages: [{ id: "WAKE", runId: "RUN", stepId: null, role: "User", status: "Completed", createdAtMs: 1,
            content: [{ type: "Text", text: wakeup }], structured: null,
            provenance: { sender: { kind: "HUMAN", id: "USER", displayName: actor, avatarUrl: null }, source: "UserMessage", correlation: null } }],
          steps: [], toolInvocations: [], interactions: [],
        } } : url.endsWith("/events") ? { events: [] } : [];
      return new Response(JSON.stringify({ code: "SUCCESS", data, message: "ok" }));
    }));
    const adapter = createInternalChatAdapter({ workspaceCode: "WORKSPACE" });
    render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><AgentChatWindow adapter={adapter} showHeader={false} config={{ floating: false }} initialSession={{ id: 1, sessionCode: "SESSION", workspaceCode: "WORKSPACE", title: "Awakening", status: "ACTIVE", createdAt: "2026-10-14T00:00:00Z", updatedAt: "2026-10-14T00:00:00Z" }} /></QueryClientProvider>);
    await screen.findByText(wakeup, { selector: ".ef-chat__message-body" });
    expect(screen.getByText(actor)).toBeTruthy();
    await waitFor(() => expect(screen.getByRole("textbox").getAttribute("placeholder")).toBe(""));
  });
});
