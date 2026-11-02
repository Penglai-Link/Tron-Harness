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
    revision: "revision",
    shares: { seed_prompt_prefix: 2500, conversation_history_summary: 500, tool_domains: 1000, current_session_history: 5000, knowledge_base: 500, "custom.plugin": 500 },
    order: ["seed_prompt_prefix", "conversation_history_summary", "tool_domains", "current_session_history", "knowledge_base", "custom.plugin"],
    blocks: [{ id: "custom.plugin", displayName: "Custom plugin", plugin: true, unavailable: false }],
    defaults: {
      order: ["seed_prompt_prefix", "conversation_history_summary", "tool_domains", "current_session_history", "custom.plugin", "knowledge_base"],
      shares: { seed_prompt_prefix: 500, conversation_history_summary: 400, tool_domains: 50, current_session_history: 7500, knowledge_base: 1500, "custom.plugin": 50 },
    },
  }),
  updateContextLayout: vi.fn().mockImplementation(async payload => payload),
}));
afterEach(cleanup);

it("shows the renamed configuration page with the same product block names as short-term memory", async () => {
  const i18n = createInstance();
  await i18n.init({ lng: "zh-CN", resources: { "zh-CN": { pages } }, defaultNS: "pages" });
  render(<I18nextProvider i18n={i18n}><ContextLayoutPage /></I18nextProvider>);
  expect(await screen.findByRole("spinbutton", { name: "对话历史摘要 的占比（%）" })).toBeTruthy();
  expect(screen.getByRole("heading", { name: "上下文配置" })).toBeTruthy();
  for (const name of ["系统提示词", "工具领域", "对话历史", "知识库信息", "Custom plugin"]) {
    expect(screen.getAllByText(name)[0]).toBeTruthy();
  }
  for (const oldName of ["会话检查点", "固定指令", "工具目录", "conversation_history_summary"]) {
    expect(screen.queryByText(oldName)).toBeNull();
  }
});

it("saves only percentage shares, validates exactly 100%, and keeps instructions first", async () => {
  const i18n = createInstance();
  await i18n.init({ lng: "zh-CN", resources: { "zh-CN": { pages } }, defaultNS: "pages" });
  render(<I18nextProvider i18n={i18n}><ContextLayoutPage /></I18nextProvider>);
  await screen.findByRole("spinbutton", { name: "对话历史摘要 的占比（%）" });
  expect(screen.queryByText("平台块")).toBeNull();
  expect(screen.queryByText("固定锚点")).toBeNull();
  expect(screen.getAllByText("插件提供")).toHaveLength(1);
  const fixed = screen.getAllByText("系统提示词").find(element => element.closest("li"))!.closest("li")!;
  expect(fixed.draggable).toBe(false);
  expect(fixed.querySelector("svg.lucide-grip-vertical")).toBeNull();
  expect(screen.queryByRole("button", { name: /^上移|^下移/ })).toBeNull();
  const movable = screen.getAllByText("Custom plugin").find(element => element.closest("li"))!.closest("li")!;
  expect(movable.draggable).toBe(true);
  fireEvent.keyDown(movable, { key: "ArrowUp", altKey: true });
  fireEvent.change(screen.getByRole("spinbutton", { name: "对话历史摘要 的占比（%）" }), { target: { value: "4" } });
  expect(screen.getByRole("button", { name: "保存" }).hasAttribute("disabled")).toBe(true);
  expect(screen.getByText("还需分配 1%")).toBeTruthy();
  fireEvent.change(screen.getByRole("spinbutton", { name: "对话历史 的占比（%）" }), { target: { value: "51" } });
  fireEvent.click(screen.getByRole("button", { name: "保存" }));
  await waitFor(() => expect(updateContextLayout).toHaveBeenCalledWith(expect.objectContaining({
    order: ["seed_prompt_prefix", "conversation_history_summary", "tool_domains", "current_session_history", "custom.plugin", "knowledge_base"],
    shares: { seed_prompt_prefix: 2500, conversation_history_summary: 400, tool_domains: 1000, current_session_history: 5100, knowledge_base: 500, "custom.plugin": 500 },
  })));
});

it("resets invalid draft shares and ordering to server defaults without saving automatically", async () => {
  vi.mocked(updateContextLayout).mockClear();
  const i18n = createInstance();
  await i18n.init({ lng: "zh-CN", resources: { "zh-CN": { pages } }, defaultNS: "pages" });
  render(<I18nextProvider i18n={i18n}><ContextLayoutPage /></I18nextProvider>);
  await screen.findByRole("spinbutton", { name: "对话历史摘要 的占比（%）" });
  fireEvent.change(screen.getByRole("spinbutton", { name: "对话历史摘要 的占比（%）" }), { target: { value: "4" } });
  expect(screen.getByRole("button", { name: "保存" }).hasAttribute("disabled")).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "重置" }));
  expect(screen.getByRole("dialog", { name: "重置上下文配置？" })).toBeTruthy();
  expect((screen.getByRole("spinbutton", { name: "对话历史摘要 的占比（%）" }) as HTMLInputElement).value).toBe("4");
  fireEvent.click(screen.getByRole("button", { name: "取消" }));
  expect(screen.queryByRole("dialog")).toBeNull();
  expect((screen.getByRole("spinbutton", { name: "对话历史摘要 的占比（%）" }) as HTMLInputElement).value).toBe("4");
  fireEvent.click(screen.getByRole("button", { name: "重置" }));
  fireEvent.click(screen.getByRole("button", { name: "确认重置" }));
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(updateContextLayout).not.toHaveBeenCalled();
  expect((screen.getByRole("spinbutton", { name: "系统提示词 的占比（%）" }) as HTMLInputElement).value).toBe("5");
  expect((screen.getByRole("spinbutton", { name: "对话历史 的占比（%）" }) as HTMLInputElement).value).toBe("75");
  expect(screen.getByText("已恢复内置默认顺序和比例，保存后生效。")).toBeTruthy();
  const names = [...screen.getByRole("list", { name: "上下文配置" }).querySelectorAll("li > div:first-child span[title]")].map(element => element.textContent);
  expect(names).toEqual(["系统提示词", "对话历史摘要", "工具领域", "对话历史", "Custom plugin", "知识库信息"]);
  expect(screen.getByRole("button", { name: "重置" }).hasAttribute("disabled")).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "保存" }));
  await waitFor(() => expect(updateContextLayout).toHaveBeenCalledWith({
    revision: "revision",
    order: ["seed_prompt_prefix", "conversation_history_summary", "tool_domains", "current_session_history", "custom.plugin", "knowledge_base"],
    shares: { seed_prompt_prefix: 500, conversation_history_summary: 400, tool_domains: 50, current_session_history: 7500, knowledge_base: 1500, "custom.plugin": 50 },
  }));
});
