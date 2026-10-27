// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { i18n } from "@/i18n";
import { ChatSubagentsPanel } from "./ChatSubagentsPanel";
import { TabVisibilityProvider } from "@/components/editable-tabs/TabVisibilityContext";

vi.mock("@/shared/api/serviceConfig", () => ({ getServiceDataSource: () => "api" }));
const requestApi = vi.hoisted(() => vi.fn());
vi.mock("@/shared/api/httpClient", () => ({ requestApi }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });
beforeEach(() => Object.defineProperty(HTMLElement.prototype, "scrollTo", {configurable:true, value:vi.fn()}));

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


it("uses the main chat work rail for one completed external command", async () => {
  await i18n.changeLanguage("zh-CN");
  const item = {runId:"child",sessionId:"temporary",parentRunId:"parent",agentCode:"coder",agentName:"Coder",task:"Run pwd",status:"Completed",startedAtMs:1,finishedAtMs:4};
  requestApi.mockImplementation(async (url: string) => url.endsWith("/child") ? {
    runId:"child",sessionId:"temporary",status:"Completed",events:[],
    records:{steps:[],interactions:[],messages:[
      {id:"task",runId:"child",stepId:null,role:"User",content:[{type:"Text",text:"Run pwd"}],structured:null,status:"Completed",createdAtMs:1},
      {id:"reply",runId:"child",stepId:"reply",role:"Assistant",content:[{type:"Text",text:"WORKDIR_TEST_OK"}],structured:{runCode:"child",finalOutput:"WORKDIR_TEST_OK",observations:[],transportPrivate:"native envelope"},status:"Completed",createdAtMs:4},
    ],toolInvocations:[{id:"command",runId:"child",stepId:"reply",messageId:"reply",callId:"item-1",toolId:"command_execution",toolName:"command_execution",arguments:{},status:"Completed",content:[],structured:{event:"item.completed",id:"item-1",status:"completed"},isError:false,resultReference:null,startedAtMs:2,finishedAtMs:3}]},
  } : [item]);
  const { container } = render(<TabVisibilityProvider visible><ChatSubagentsPanel sessionCode="parent" workspaceCode="workspace" /></TabVisibilityProvider>);
  fireEvent.click(await screen.findByRole("button", {name:/Coder/}));
  await screen.findByText("WORKDIR_TEST_OK");
  expect(container.querySelector(".ef-chat__composer")).toBeNull();
  expect(container.querySelectorAll(".ef-chat__rail-summary")).toHaveLength(1);
  const rail = container.querySelector<HTMLButtonElement>(".ef-chat__rail-summary")!;
  expect(rail.getAttribute("aria-expanded")).toBe("false");
  fireEvent.click(rail);
  expect(container.querySelectorAll(".ef-chat__tool-toggle")).toHaveLength(1);
  expect(container.querySelector(".ef-chat__tool-title")?.textContent).toBe("执行命令");
  expect(container.querySelector(".ef-chat__tool-badge--done")).not.toBeNull();
  expect(screen.queryByText("结果未知")).toBeNull();
  expect(container.textContent).not.toContain("transportPrivate");
});
