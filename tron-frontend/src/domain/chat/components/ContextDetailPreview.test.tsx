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
