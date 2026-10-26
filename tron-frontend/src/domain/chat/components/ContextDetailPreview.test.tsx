// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { i18n } from "@/i18n";
import { ContextDetailPreview, contextDetailRawValue } from "./ContextDetailPreview";

afterEach(cleanup);

it("keeps raw prompt content separate from budget and provenance reports", () => {
  const messages = [{ role: "system", parts: [{ type: "text", text: "System instructions" }] }];
  expect(contextDetailRawValue("systemPrompt", { messages, budgetBreakdown: { promptPrefixTokens: 6906 }, blocks: [{}] })).toEqual(messages);
});

describe.each(["zh-CN", "en-US"])("context inspector in %s", (locale) => {
  it("renders runtime data with readable labels and omits empty control fields", async () => {
    await i18n.changeLanguage(locale);
    const messages = [{ role: "user", parts: [{ type: "text", text: JSON.stringify({
      userCode: "user-1", workspaceCode: "", toolCount: 0,
      eventKind: "USER_PROMPT", event: { annotations: [], requestedSkills: [], resume: [] },
      relatedWork: [], nameConfirmed: false,
    }) }] }];
    const { container } = render(<ContextDetailPreview block="dude_environment" value={{ messages }} />);
    expect(screen.getByText(locale === "zh-CN" ? "用户身份" : "User identity")).toBeTruthy();
    expect(screen.getByText(locale === "zh-CN" ? "普通用户消息" : "User message")).toBeTruthy();
    expect(screen.getByText("0")).toBeTruthy();
    expect(screen.getByText(locale === "zh-CN" ? "否" : "No")).toBeTruthy();
    expect(container.textContent).not.toContain("annotations");
    expect(container.textContent).not.toContain("requestedSkills");
    expect(container.textContent).not.toContain("resume");
    expect(contextDetailRawValue("dude_environment", { messages })).toEqual(messages);
  });

  it("does not misread an inner scope as a full truncated runtime document", async () => {
    await i18n.changeLanguage(locale);
    const text = '{"userCode":"user-1","scope":{"applications":[]},"availableTools":["tool_a","tool_b';
    const messages = [{ role: "user", parts: [{ type: "text", text: `[Runtime reference data: dude_environment]\n${text}` }] }];
    const { container } = render(<ContextDetailPreview block="dude_environment" value={{ messages }} />);
    expect(screen.getByRole("note").textContent).toContain(locale === "zh-CN" ? "已被截断" : "truncated");
    expect(container.querySelector("pre")?.textContent).toBe(text);
    expect(container.querySelector("dl")).toBeNull();
    expect(contextDetailRawValue("dude_environment", { messages })).toEqual(messages);
  });

  it("previews trigger evidence separately from instructions and keeps bracketed prose readable", async () => {
    await i18n.changeLanguage(locale);
    const { container } = render(<ContextDetailPreview block="dude_wakeup" value={{ messages: [{ role: "user", parts: [{ type: "text", text: JSON.stringify({
      event: { source: "ThreadFeedback", taskCode: "task-1", payload: { summary: "[Thread] needs an answer" } },
      relatedWork: [{ summary: "Confirm a release", requestedAction: "Choose a time", evidence: ["interaction-1"] }],
    }) }] }] }} />);
    expect(screen.getByText(locale === "zh-CN" ? "后台执行反馈" : "Background execution feedback")).toBeTruthy();
    expect(screen.getByText("[Thread] needs an answer")).toBeTruthy();
    expect(screen.getByText("Confirm a release")).toBeTruthy();
    expect(screen.getByText("interaction-1")).toBeTruthy();
    expect(screen.queryByRole("note")).toBeNull();
    expect(container.textContent).not.toContain("dude_wakeup");
  });

  it("does not mistake bracketed instructions for truncated JSON", async () => {
    await i18n.changeLanguage(locale);
    render(<ContextDetailPreview block="systemPrompt" value={{ messages: [{ role: "system", parts: [{ type: "text", text: "[Role] You are an assistant." }] }] }} />);
    expect(screen.queryByRole("note")).toBeNull();
    expect(screen.getByText("[Role] You are an assistant.")).toBeTruthy();
  });

  it.each(["context", "wrappedText", "json", "boundedText"])("makes tool domains readable from %s without changing raw data", async (format) => {
    await i18n.changeLanguage(locale);
    const domains = [{ domainCode: "app", name: "Application tools", description: "Manage workspace applications.", toolCount: 5, sampleTools: ["app_page"], hasMutationTools: true }];
    const context = { type: "context", kind: "tool_domains", text: JSON.stringify(domains) };
    const part = format === "context" ? context
      : format === "json" ? { type: "Json", value: { tronMessagePart: context } }
      : { type: "text", text: format === "wrappedText" ? JSON.stringify({ tronMessagePart: context })
        : `[Runtime reference data: tool_domains]\n${JSON.stringify(domains)}` };
    const messages = [{ role: "user", parts: [part] }];
    const { container } = render(<ContextDetailPreview block="tool_domains" value={{ messages }} />);
    expect(screen.getByText("Application tools")).toBeTruthy();
    expect(screen.getByText("Manage workspace applications.")).toBeTruthy();
    expect(container.textContent).toContain(locale === "zh-CN" ? "5 个工具" : "5 tools");
    expect(container.textContent).toContain(locale === "zh-CN" ? "含修改工具" : "Can modify");
    expect(container.querySelector("details")?.open).toBe(false);
    expect(screen.getByText("app_page")).toBeTruthy();
    expect(container.textContent).not.toContain("tronMessagePart");
    expect(container.textContent).not.toContain("domainCode");
    expect(contextDetailRawValue("tool_domains", { messages })).toEqual(messages);
  });

  it("unwraps typed context in other contributor previews", async () => {
    await i18n.changeLanguage(locale);
    const part = { type: "context", kind: "current_session_working_state", text: "Current goal: review this project" };
    const { container } = render(<ContextDetailPreview block="current_session_working_state" value={{ messages: [
      { role: "user", parts: [{ type: "text", text: JSON.stringify({ tronMessagePart: part }) }] },
    ] }} />);
    expect(container.textContent).toContain("Current goal: review this project");
    expect(container.textContent).not.toContain("tronMessagePart");
  });

  it("labels truncated historical domain snapshots and hides the transport wrapper", async () => {
    await i18n.changeLanguage(locale);
    const domains = [{ domainCode: "app", name: "Application tools", description: "Manage applications", toolCount: 5, sampleTools: ["app_page"] }, { domainCode: "incomplete", name: "Missing details" }];
    const encoded = JSON.stringify({ tronMessagePart: { type: "context", kind: "tool_domains", text: JSON.stringify(domains) } });
    const text = encoded.slice(0, encoded.indexOf("incomplete") + 3);
    const messages = [{ role: "user", parts: [{ type: "text", text }] }];
    const { container } = render(<ContextDetailPreview block="tool_domains" value={{ messages }} />);
    expect(screen.getByText("Application tools")).toBeTruthy();
    expect(screen.getByRole("note").textContent).toContain(locale === "zh-CN" ? "已被截断" : "truncated");
    expect(container.textContent).not.toContain("Missing details");
    expect(container.textContent).not.toContain("tronMessagePart");
    expect(contextDetailRawValue("tool_domains", { messages })).toEqual(messages);
  });

  it("reads the native tool envelope and shows its actual name and parameter schema", async () => {
    await i18n.changeLanguage(locale);
    render(<ContextDetailPreview block="toolSchema" value={{ blockCode: "toolSchema", tools: [{ toolName: "chat_ask_user", description: "Ask a question", inputSchema: { type: "object", required: ["questions"], properties: { questions: { type: "array", description: "Questions for the user" } } } }], budgetBreakdown: { toolSchemaTokens: 435 } }} />);
    expect(screen.getByRole("heading", { name: "chat_ask_user" })).toBeTruthy();
    expect(screen.getByRole("table").textContent).toContain("Questions for the user");
    expect(screen.getByRole("table").textContent).toContain("array");
    expect(screen.getByLabelText(locale === "zh-CN" ? "必填" : "Required")).toBeTruthy();
    expect(screen.queryByText("budgetBreakdown")).toBeNull();
  });

  it("renders system message parts directly with flat sections", async () => {
    await i18n.changeLanguage(locale);
    const { container } = render(<ContextDetailPreview block="systemPrompt" value={{ messages: [{ role: "system", parts: [{ type: "text", text: 'System instructions\n\nAvailable domains:\n[{"name":"Filesystem","description":"Read files","sampleTools":["read_file"]}]' }] }], blocks: [{ id: "framework.instructions" }], budgetBreakdown: { promptPrefixTokens: 6906 } }} />);
    expect(screen.getByText("System instructions")).toBeTruthy();
    expect(screen.getByText("Filesystem")).toBeTruthy();
    expect(screen.getByText("read_file")).toBeTruthy();
    expect(container.querySelectorAll("article")).toHaveLength(0);
    expect(container.textContent).not.toContain("framework.instructions");
    expect(container.textContent).not.toContain("6906");
  });

  it("previews the native application-context messages from the saved request", async () => {
    await i18n.changeLanguage(locale);
    const messages = [
      { role: "user", parts: [{ type: "text", text: '<available_tool_domains>[{"domainCode":"app","name":"Application tools","description":"Manage workspace applications.","toolCount":5,"sampleTools":["app.page"],"hasMutationTools":true},{"domainCode":"git","name":"Git tools","description":"Inspect branches.","toolCount":2,"sampleTools":["git.status"],"hasMutationTools":false}]</available_tool_domains>' }] },
      { role: "user", parts: [{ type: "text", text: "Current workspace: demo" }] },
    ];
    const { container } = render(<ContextDetailPreview block="applicationContext" value={{ messages }} />);
    expect(container.textContent).toContain("Application tools");
    expect(container.textContent).toContain("app.page");
    expect(container.textContent).toContain("Current workspace: demo");
    expect(container.querySelectorAll("details")).toHaveLength(2);
    expect(container.querySelector("details")?.open).toBe(false);
    expect(container.textContent).not.toContain("domainCode");
    expect(container.textContent).not.toContain("sampleTools");
    expect(container.textContent).not.toContain(i18n.t("chat.applications.noSnapshot", { ns: "pages" }));
    expect(contextDetailRawValue("applicationContext", { messages })).toEqual(messages);
  });

  it("groups the execution scope separately from tool domains", async () => {
    await i18n.changeLanguage(locale);
    render(<ContextDetailPreview block="applicationContext" value={{ messages: [{ role: "user", parts: [{ type: "context", text: `<application-execution-context>\nRuntime metadata\n${JSON.stringify({ localDirectory: "/workspace/project", applications: [] })}\n</application-execution-context>` }] }] }} />);
    expect(screen.getByRole("heading", { name: i18n.t("chat.applications.preview.executionLocation", { ns: "pages" }) })).toBeTruthy();
    expect(screen.getByText("/workspace/project")).toBeTruthy();
  });

  it("makes the four native history messages and hidden interaction answer readable", async () => {
    await i18n.changeLanguage(locale);
    const { container } = render(<ContextDetailPreview block="history" value={{ messages: [
      { role: "user", parts: [{ type: "text", text: "你好" }] },
      { role: "assistant", parts: [{ type: "toolCall", tool_name: "chat_ask_user", input: { questions: ["What should I do?"] } }] },
      { role: "tool", parts: [{ type: "toolResult", tool_name: "chat_ask_user", output: "Waiting for an answer" }] },
      { role: "user", parts: [{ type: "context", text: JSON.stringify({ interaction_id: "internal-id", answer: { answers: { id: { text: "什么都不做" } } } }) }] },
    ] }} />);
    expect(screen.getAllByRole("heading")).toHaveLength(4);
    expect(screen.getByText("什么都不做")).toBeTruthy();
    expect(container.textContent).toContain(locale === "zh-CN" ? "工具调用" : "Tool call");
    expect(container.textContent).toContain(locale === "zh-CN" ? "工具结果" : "Tool result");
    expect(container.textContent).not.toContain("internal-id");
  });
});
