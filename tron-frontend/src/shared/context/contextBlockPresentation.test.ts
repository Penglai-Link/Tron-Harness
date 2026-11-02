import { describe, expect, it } from "vitest";
import { createInstance } from "i18next";
import { pages as enPages } from "@/i18n/locales/en-US/pages";
import { pages as zhPages } from "@/i18n/locales/zh-CN/pages";
import { contextBlockName } from "./contextBlockPresentation";

describe("shared context block names", () => {
  for (const [locale, pages, expected] of [
    ["zh-CN", zhPages, ["系统提示词", "对话历史摘要", "工具领域", "对话历史", "工作状态", "知识库信息"]],
    ["en-US", enPages, ["System Prompt", "Conversation Summary", "Tool Domains", "Conversation History", "Working State", "Knowledge Base Information"]],
  ] as const) {
    it(`uses the short-term memory names in ${locale} and retains plugin metadata`, async () => {
      const i18n = createInstance();
      await i18n.init({ lng: locale, resources: { [locale]: { pages } }, defaultNS: "pages" });
      const ids = ["seed_prompt_prefix", "conversation_history_summary", "tool_domains", "current_session_history", "current_session_working_state", "knowledge_base"];
      expect(ids.map(id => contextBlockName(id, i18n.t))).toEqual(expected);
      expect(contextBlockName("framework.history", i18n.t)).toBe(contextBlockName("current_session_history", i18n.t));
      expect(contextBlockName("framework.instructions", i18n.t)).toBe(contextBlockName("seed_prompt_prefix", i18n.t));
      expect(contextBlockName("custom.plugin", i18n.t, "Custom plugin")).toBe("Custom plugin");
      expect(contextBlockName("unknown", i18n.t)).toBe("unknown");
      expect(contextBlockName("current_session_checkpoint", i18n.t)).toBe(contextBlockName("conversation_history_summary", i18n.t));
    });
  }
});
