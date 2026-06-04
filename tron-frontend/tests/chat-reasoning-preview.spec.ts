import { expect, test } from "@playwright/test";

import { deriveReasoningPreview } from "../src/domain/chat/pages/chatReasoningPreview";

test("reasoning preview keeps a readable Chinese first sentence", () => {
  const preview = deriveReasoningPreview(
    "我需要先梳理用户上下文，再确认工具调用顺序。然后再进入执行阶段。",
    [],
  );

  expect(preview).toBe("我需要先梳理用户上下文，再确认工具调用顺序。");
});

test("reasoning preview falls back to the full detail when the clause is too short", () => {
  const preview = deriveReasoningPreview(
    "\" in Chinese. The user asked \"我是谁\" and I should answer in Chinese with enough context to be useful.",
    [],
  );

  expect(preview).toContain("我是谁");
  expect(preview).toContain("answer in Chinese");
  expect(preview.length).toBeGreaterThan(40);
});

test("reasoning preview appends tool counts without losing the main detail", () => {
  const preview = deriveReasoningPreview(
    "Reviewing session context and available skills. Additional detail follows.",
    [{ started: true }, { started: true }],
  );

  expect(preview).toContain("Reviewing session context and available skills.");
  expect(preview).toContain("Used 2 tools.");
});
