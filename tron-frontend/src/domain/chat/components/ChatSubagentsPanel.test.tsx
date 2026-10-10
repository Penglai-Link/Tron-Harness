// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { i18n } from "@/i18n";
import { ChatSubagentsPanel } from "./ChatSubagentsPanel";
import { TabVisibilityProvider } from "@/components/editable-tabs/TabVisibilityContext";

vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "api" }));
const requestApi = vi.hoisted(() => vi.fn());
vi.mock("@/shared/api/httpClient", () => ({ requestApi }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });

it.each([["zh-CN", "运行中", "返回子代理列表"], ["en-US", "Running", "Back to subagents"]])(
  "loads only summaries until a child is opened in %s", async (locale, status, back) => {
    await i18n.changeLanguage(locale);
    const items = [{ runId: "child", sessionId: "temporary", parentRunId: "parent", agentCode: "reviewer", agentName: "Reviewer", avatarId: "fox", task: "Review migrations", status: "Running", startedAtMs: 1, finishedAtMs: null }];
    requestApi.mockImplementation(async (url: string) => url.endsWith("/child") ? {
      runId: "child", sessionId: "temporary", status: "Completed", events: [],
      records: { steps: [], toolInvocations: [], interactions: [], messages: [{ id: "reply", runId: "child", stepId: null, role: "Assistant", content: [{ type: "Text", text: "Migrations reviewed" }], structured: null, status: "Completed", createdAtMs: 2 }] },
    } : items);
    const view = render(<TabVisibilityProvider visible><ChatSubagentsPanel sessionCode="parent-chat" workspaceCode="workspace" /></TabVisibilityProvider>);
    await screen.findByText("Reviewer");
    const avatar = screen.getByRole("img", { name: `Reviewer · ${status}` });
    expect(avatar.getAttribute("data-status")).toBe("Running");
    expect(avatar.querySelector("img")?.getAttribute("src")).toContain("fox.png");
    expect(screen.queryByText(status)).toBeNull();
    expect(requestApi.mock.calls.every(([url]) => url.endsWith("/subagents"))).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: /Reviewer/ }));
    await screen.findByText("Migrations reviewed");
    expect(requestApi).toHaveBeenCalledWith("/api/v1/chats/sessions/parent-chat/subagents/child", expect.objectContaining({ queryParams: { workspaceCode: "workspace" } }));
    const childSignal = requestApi.mock.calls.find(([url]) => url.endsWith("/child"))![1].signal as AbortSignal;
    view.rerender(<TabVisibilityProvider visible={false}><ChatSubagentsPanel sessionCode="parent-chat" workspaceCode="workspace" /></TabVisibilityProvider>);
    expect(childSignal.aborted).toBe(true);
    view.rerender(<TabVisibilityProvider visible><ChatSubagentsPanel sessionCode="parent-chat" workspaceCode="workspace" /></TabVisibilityProvider>);
    await waitFor(() => expect(requestApi.mock.calls.filter(([url]) => url.endsWith("/child"))).toHaveLength(2));
    const resumedSignal = requestApi.mock.calls.filter(([url]) => url.endsWith("/child"))[1][1].signal as AbortSignal;
    fireEvent.click(screen.getByRole("button", { name: back }));
    expect(resumedSignal.aborted).toBe(true);
    expect(screen.queryByText("Migrations reviewed")).toBeNull();
    view.rerender(<ChatSubagentsPanel sessionCode="another-chat" workspaceCode="workspace" />);
    await waitFor(() => expect(requestApi).toHaveBeenCalledWith("/api/v1/chats/sessions/another-chat/subagents", expect.anything()));
    expect(screen.queryByText("Migrations reviewed")).toBeNull();
  },
);
