import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/knowledge-base/kb-notices", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const locale of ["en-US", "zh-CN"] as const) {
  test(`Document notices stay behind a warning icon in ${locale}`, async ({ page }, testInfo) => {
    const zh = locale === "zh-CN";
    await routeResourceUiShellApi(page);
    await seedResourceUiShell(page, locale);
    await page.route("**/api/v1/chats/session-groups?*", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: [] } }));
    const config = { mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2 };
    const base = { resourceCode: "res-notice", resourceRevision: 1, status: "READY", error: null, chunkCount: 3, indexRevision: 1, chunkConfig: config, parsingTier: "standard" };
    const imageAnalysis = { imageCount: 1, originalsOnly: 0, withSourceAnnotations: 1, withGeneratedDescriptions: 0, withUnattributedBodyText: 0, generationMode: "disabled_by_tier" };
    await page.route("**/api/v1/knowledge-bases**", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: {
      id: "kb-notices", name: "Guidelines", workspaceCode: "TRON_CORE", agentCodes: [], documentCount: 4, defaultChunkConfig: config,
      documents: [
        { ...base, id: "doc-image", fileName: "guideline.pdf", imageAnalysis, warnings: ["图片原件和位置已保留；图片语义描述需要单独验证"] },
        { ...base, id: "doc-clean", fileName: "clean.pdf" },
        { ...base, id: "doc-empty", fileName: "empty-notice.pdf", imageAnalysis: { ...imageAnalysis, withSourceAnnotations: 0, generationMode: "parser_controlled" }, warnings: ["图片原件和位置已保留；图片语义描述需要单独验证"] },
        { ...base, id: "doc-error", fileName: "failed.pdf", status: "FAILED", error: "Parser unavailable", warnings: ["Document page is incomplete"] },
      ],
    } } }));
    await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-notices`);
    const row = page.getByRole("row").filter({ hasText: "guideline.pdf" });
    const nameCell = row.locator("td").nth(1);
    const notice = nameCell.getByRole("button", { name: zh ? "guideline.pdf 的处理说明" : "Processing details for guideline.pdf" });
    const tooltip = page.locator('[role="tooltip"][data-open]');
    await expect(nameCell).toHaveText("guideline.pdf");
    await expect(tooltip).toHaveCount(0);
    await expect(page.getByRole("row").filter({ hasText: "clean.pdf" }).locator("td").nth(1).getByRole("button")).toHaveCount(0);
    await expect(page.getByRole("row").filter({ hasText: "empty-notice.pdf" }).locator("td").nth(1).getByRole("button")).toHaveCount(0);
    await notice.hover();
    await expect(tooltip).toContainText(zh ? "1 张图片含原文图注或脚注" : "1 image(s) have original captions or footnotes");
    await expect(tooltip).toContainText(zh ? "当前解析质量不生成图片描述" : "The current parsing quality does not generate image descriptions");
    await expect(tooltip).not.toContainText(zh ? "图片语义描述需要单独验证" : "Image descriptions need separate verification");
    await page.screenshot({ animations: "disabled", path: testInfo.outputPath(`document-notices-${locale}.png`) });
    await page.mouse.move(1, 1);
    await expect(tooltip).toHaveCount(0);
    await notice.focus();
    await expect(tooltip).toHaveCount(0);
    await notice.hover();
    await expect(tooltip).toContainText(zh ? "当前解析质量" : "The current parsing quality");
    await notice.press("Escape");
    await expect(tooltip).toHaveCount(0);
    const errorCell = page.getByRole("row").filter({ hasText: "failed.pdf" }).locator("td").nth(1);
    await expect(errorCell).toHaveText("failed.pdf");
    await errorCell.getByRole("button").hover();
    await expect(tooltip).toContainText("Parser unavailable");
    await expect(tooltip).toContainText("Document page is incomplete");
  });
}
