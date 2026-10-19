// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { pages } from "@/i18n/locales/en-US/pages";
import { pages as chinesePages } from "@/i18n/locales/zh-CN/pages";
import { currentImageWarnings, ImageAnalysisStatus } from "./ImageAnalysisStatus";
const locale = vi.hoisted(() => ({ chinese: false }));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string, args?: Record<string, unknown>) => {
  const value = key.split(".").reduce((node: unknown, item) => (node as Record<string, unknown>)?.[item], locale.chinese ? chinesePages : pages);
  return String(value).replace(/{{(\w+)}}/g, (_, k) => String(args?.[k] ?? ""));
} }) }));
afterEach(() => { cleanup(); locale.chinese = false; });
const summary = { imageCount: 3, originalsOnly: 1, withSourceAnnotations: 1, withGeneratedDescriptions: 0, withUnattributedBodyText: 1, generationMode: "disabled_by_tier" as const };
describe("ImageAnalysisStatus", () => {
 it("removes resolved historical image notices while retaining genuine parse warnings", () => {
  const old = "3 张图片没有可用描述或 OCR 文字，已保留解析产物但未建立图像语义索引";
  const warnings = [old, "table row larger than chunk budget"];
  expect(currentImageWarnings(warnings, { ...summary, originalsOnly: 0, withGeneratedDescriptions: 3, supplementStatus: "ready" })).toEqual([warnings[1]]);
  expect(currentImageWarnings(warnings, { ...summary, supplementStatus: "pending" })).toEqual(warnings);
  expect(currentImageWarnings(warnings, { ...summary, imageCount: 0 })).toEqual([warnings[1]]);
 });
 it("shows only actual sources and explains the configured capability", () => {
  const { container } = render(<ImageAnalysisStatus summary={summary} />);
  expect(screen.getByText("1 image(s) saved without accompanying descriptions.")).toBeTruthy();
  expect(screen.getByText("1 image(s) have original captions or footnotes.")).toBeTruthy();
  expect(screen.getByText("1 image(s) have accompanying text without model-generation provenance.")).toBeTruthy();
  expect(screen.getByText("The current parsing quality does not generate image descriptions.")).toBeTruthy();
  expect(container.textContent).not.toContain("verification");
  expect(container.textContent).not.toContain("have model-generated descriptions");
 });
 it("shows generated output only when supplied by the backend", () => {
  render(<ImageAnalysisStatus summary={{ ...summary, originalsOnly: 0, withSourceAnnotations: 0, withUnattributedBodyText: 0, withGeneratedDescriptions: 3, generationMode: "parser_controlled" }} />);
  expect(screen.getByText("3 image(s) have model-generated descriptions.")).toBeTruthy();
  expect(screen.queryByText("The current parsing quality does not generate image descriptions.")).toBeNull();
 });
 it("has accurate Chinese wording and no internal acceptance tasks", () => {
  locale.chinese = true;
  const { container } = render(<ImageAnalysisStatus summary={summary} />);
  expect(screen.getByText("1 张图片含原文图注或脚注。")).toBeTruthy();
  expect(container.textContent).not.toMatch(/验证|验收|测试/);
  expect(chinesePages.knowledgeLibrary.resourcesHelp).not.toMatch(/验证|验收|测试/);
  expect(chinesePages.knowledgeLibrary.imageNotice).not.toMatch(/验证|验收|测试/);
 });
 it("shows configured supplement pending without claiming output or parser generation", () => {
  render(<ImageAnalysisStatus summary={{ ...summary, supplementStatus: "pending" }} />);
  expect(screen.getByText("Image descriptions will be generated when writing the index.")).toBeTruthy();
  expect(screen.queryByText("The current parsing quality does not generate image descriptions.")).toBeNull();
 });
 it("does not infer image capability from absent metadata", () => {
  const { container } = render(<ImageAnalysisStatus />);
  expect(container.textContent).toBe("");
 });
});
