// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { createInstance } from "i18next";
import { I18nextProvider } from "react-i18next";
import type { ReactNode } from "react";
import { pages } from "@/i18n/locales/zh-CN/pages";
import { updateContextLayout } from "../api/contextLayoutApi";
import { ContextLayoutPage } from "./ContextLayoutPage";

vi.mock("@/shared", () => ({
  PageHeader: ({ title, actions }: { title: string; actions: ReactNode }) => <header><h1>{title}</h1>{actions}</header>,
  SurfaceCard: ({ children }: { children: ReactNode }) => <section>{children}</section>,
}));
vi.mock("../api/contextLayoutApi", () => ({
  fetchContextLayout: vi.fn().mockResolvedValue({
    revision: "revision", pluginOverrides: {},
    nativeBudgets: { conversation_history_summary: { maxTokens: 6000, maxRatioBps: 2000 }, knowledge_base: { maxTokens: 1200, maxRatioBps: null } },
    order: ["seed_prompt_prefix", "conversation_history_summary", "tool_domains", "current_session_history", "knowledge_base", "custom.plugin"],
    blocks: [{ id: "custom.plugin", displayName: "Custom plugin", plugin: true, unavailable: false }],
  }),
  updateContextLayout: vi.fn().mockImplementation(async payload => payload),
}));
afterEach(cleanup);

it("shows the renamed configuration page with the same product block names as short-term memory", async () => {
  const i18n = createInstance();
  await i18n.init({ lng: "zh-CN", resources: { "zh-CN": { pages } }, defaultNS: "pages" });
  render(<I18nextProvider i18n={i18n}><ContextLayoutPage /></I18nextProvider>);
  expect(await screen.findByText("对话历史摘要")).toBeTruthy();
  expect(screen.getByRole("heading", { name: "上下文配置" })).toBeTruthy();
  for (const name of ["系统提示词", "工具领域", "对话历史", "知识库信息", "Custom plugin"]) {
    expect(screen.getByText(name)).toBeTruthy();
  }
  for (const oldName of ["会话检查点", "固定指令", "工具目录", "conversation_history_summary"]) {
    expect(screen.queryByText(oldName)).toBeNull();
  }
  expect(screen.getByText("custom.plugin")).toBeTruthy();
});

it("edits host budgets, labels only plugins, and prevents moving fixed rows", async () => {
  const i18n = createInstance();
  await i18n.init({ lng: "zh-CN", resources: { "zh-CN": { pages } }, defaultNS: "pages" });
  render(<I18nextProvider i18n={i18n}><ContextLayoutPage /></I18nextProvider>);
  await screen.findByText("对话历史摘要");
  expect(screen.queryByText("平台块")).toBeNull();
  expect(screen.queryByText("固定锚点")).toBeNull();
  expect(screen.getAllByText("插件提供")).toHaveLength(1);
  const fixed = screen.getByText("对话历史摘要").closest("li")!;
  expect(fixed.draggable).toBe(false);
  expect(fixed.querySelector("svg.lucide-grip-vertical")).toBeNull();
  expect(screen.queryByRole("button", { name: /^上移|^下移/ })).toBeNull();
  const movable = screen.getByText("Custom plugin").closest("li")!;
  expect(movable.draggable).toBe(true);
  fireEvent.keyDown(movable, { key: "ArrowUp", altKey: true });
  fireEvent.change(screen.getByRole("spinbutton", { name: "对话历史摘要 的最大 tokens" }), { target: { value: "4000" } });
  fireEvent.change(screen.getByRole("spinbutton", { name: "对话历史摘要 的最大占比（%）" }), { target: { value: "15" } });
  fireEvent.change(screen.getByRole("spinbutton", { name: "知识库信息 的最大 tokens" }), { target: { value: "2500" } });
  fireEvent.click(screen.getByRole("button", { name: "保存" }));
  await waitFor(() => expect(updateContextLayout).toHaveBeenCalledWith(expect.objectContaining({
    order: ["seed_prompt_prefix", "conversation_history_summary", "tool_domains", "current_session_history", "custom.plugin", "knowledge_base"],
    nativeBudgets: { conversation_history_summary: { maxTokens: 4000, maxRatioBps: 1500 }, knowledge_base: { maxTokens: 2500, maxRatioBps: null } },
  })));
});
