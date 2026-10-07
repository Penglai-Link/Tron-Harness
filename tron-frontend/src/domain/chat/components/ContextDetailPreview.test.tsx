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

  it("shows an explicit empty family state instead of rendering report internals", async () => {
    await i18n.changeLanguage(locale);
    const { container } = render(<ContextDetailPreview block="familyContext" value={{ blockCode: "familyContext", messages: [], blocks: [{ id: "session_family_context", source_version: "hash:0", final_message_count: 0 }], budgetBreakdown: {} }} />);
    expect(screen.getByText(locale === "zh-CN" ? "没有加载相关会话摘要。" : "No related session summaries were loaded.")).toBeTruthy();
    expect(container.textContent).not.toContain("session_family_context");
    expect(container.textContent).not.toContain("hash:0");
  });

  it("shows the authorized family summary and coverage without technical envelopes", async () => {
    await i18n.changeLanguage(locale);
    const { container } = render(<ContextDetailPreview block="familyContext" value={{ messages: [{ role: "user", parts: [{ type: "context", kind: "session_family_context", text: JSON.stringify({ sessionCode: "session-a-12345", summaryCode: "opaque-hash", summary: "Remember sapphire", coveredMessages: 4, createdAt: 1791350400000, sourceRevision: 6 }) }] }], blocks: [{ id: "session_family_context" }] }} />);
    expect(screen.getByText("Remember sapphire")).toBeTruthy();
    expect(container.textContent).toContain(locale === "zh-CN" ? "摘要覆盖 4 条消息" : "Summary covers 4 messages");
    expect(container.textContent).not.toContain("opaque-hash");
    expect(container.querySelectorAll("article")).toHaveLength(0);
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
