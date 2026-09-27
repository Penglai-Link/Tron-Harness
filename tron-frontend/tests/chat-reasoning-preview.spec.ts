import { expect, test } from "@playwright/test";
import { deriveReasoningPreview } from "../src/domain/chat/pages/chatReasoningPreview";

test("reasoning preview preserves the supplied summary", () => {
  const summary = "先梳理用户上下文，再确认工具调用顺序。然后进入执行阶段。";
  expect(deriveReasoningPreview(summary)).toBe(summary);
});

test("reasoning preview normalizes markdown and whitespace", () => {
  expect(deriveReasoningPreview("**Reviewing** context.\n\n  Checking tools.")).toBe("Reviewing context. Checking tools.");
});

test("reasoning preview truncates long summaries without adding tool counts", () => {
  const preview = deriveReasoningPreview("Reviewing context and available skills. ".repeat(10));
  expect(preview.length).toBeLessThanOrEqual(121);
  expect(preview).toContain("Reviewing context");
  expect(preview).not.toContain("Used");
});
