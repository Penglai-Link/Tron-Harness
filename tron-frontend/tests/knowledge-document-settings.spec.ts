import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/knowledge-base/kb-help", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const locale of ["en-US", "zh-CN"] as const) {
  test(`Document settings open quietly and explain optional auto parameters in ${locale}`, async ({ page }, testInfo) => {
    const zh = locale === "zh-CN";
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    await page.route("**/api/v1/chats/session-groups?*", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: [] } }));
    const config = { mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2 };
    const document = { id: "doc-help", name: "Guideline", fileName: "guideline.pdf", resourceCode: "res-help", resourceRevision: 1, status: "READY", error: null, chunkCount: 3, indexRevision: 1, chunkConfig: config, parsingTier: "standard" };
    const updates: unknown[] = [];
    await page.route("**/api/v1/knowledge-bases**", async route => {
      const path = new URL(route.request().url()).pathname;
      let data: unknown = { id: "kb-help", name: "Guidelines", workspaceCode: "TRON_CORE", documents: [document], documentCount: 1, agentCodes: [], defaultChunkConfig: config };
      if (path.endsWith("/parsing-capabilities")) data = { defaultOcrMode: "Ocr", defaultTier: "standard", availableTiers: ["flash", "basic", "standard", "advanced"], parserVersion: "4.0.10", officeTier: "flash" };
      if (route.request().method() === "PATCH") {
        expect(route.request().headers()["accept-language"]).toBe(locale);
        const body = route.request().postDataJSON(); updates.push(body);
        data = { ...document, status: "QUEUED", chunkConfig: body.chunkConfig };
      }
      await route.fulfill({ json: { code: "SUCCESS", message: "ok", data } });
    });
    await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-help`);
    await page.getByRole("row").filter({ hasText: "guideline.pdf" }).getByRole("button").click();
    await page.getByRole("menuitem", { name: zh ? "设置" : "Settings", exact: true }).click();
    const dialog = page.getByRole("dialog");
    const tooltip = page.locator('[role="tooltip"][data-open]');
    await expect(dialog.getByRole("heading", { name: "guideline.pdf" })).toBeFocused();
    await expect(tooltip).toHaveCount(0);
    const max = dialog.getByRole("spinbutton", { name: zh ? "每段最大字符数" : "Maximum characters per chunk", exact: true });
    const overlap = dialog.getByRole("spinbutton", { name: zh ? "重叠字符数" : "Overlap characters", exact: true });
    await expect(max).not.toBeVisible();
    await expect(dialog.getByRole("button", { name: zh ? "保存" : "Save", exact: true })).toBeDisabled();
    await dialog.getByRole("button", { name: zh ? "解析质量选项说明" : "Parsing quality options" }).hover();
    await expect(tooltip).toContainText(zh ? "扫描 PDF" : "scanned PDFs");
    await dialog.getByRole("button", { name: zh ? "切分方式选项说明" : "Chunking mode options" }).hover();
    await expect(tooltip).toContainText(zh ? "自动" : "Automatic");
    await expect(tooltip).toContainText(zh ? "按标题" : "By heading");
    await expect(tooltip).toContainText(zh ? "固定长度" : "Fixed length");
    await page.mouse.move(1, 1);
    await page.screenshot({ animations: "disabled", path: testInfo.outputPath(`document-settings-${locale}.png`) });
    await dialog.locator("summary").click();
    await expect(max).toBeVisible(); await expect(max).toHaveValue("1200"); await expect(overlap).toHaveValue("120");
    await dialog.getByRole("switch").click();
    await expect(max).toBeDisabled();
    await dialog.getByRole("button", { name: zh ? "切分方式选项说明" : "Chunking mode options" }).hover();
    await expect(tooltip).toContainText(zh ? "自动" : "Automatic");
    await dialog.getByRole("switch").click();
    const mode = dialog.getByRole("combobox", { name: zh ? "切分方式" : "Splitting", exact: true });
    await mode.click(); await page.getByRole("option", { name: zh ? "固定长度" : "Fixed length", exact: true }).click();
    await expect(max).toBeVisible(); await expect(dialog.locator("summary")).toHaveCount(0);
    await mode.click(); await page.getByRole("option", { name: zh ? "按标题" : "By heading", exact: true }).click();
    await expect(dialog.getByRole("spinbutton", { name: zh ? "标题深度" : "Heading depth", exact: true })).toBeVisible();
    await mode.click(); await page.getByRole("option", { name: zh ? "自动" : "Automatic", exact: true }).click();
    await expect(max).not.toBeVisible();
    const ocr = dialog.getByRole("combobox", { name: zh ? "文字识别方式" : "Text recognition", exact: true });
    await expect(ocr).toContainText(zh ? "继承平台（强制 OCR）" : "Use platform default (Force OCR)");
    await dialog.getByRole("button", { name: zh ? "文字识别方式选项说明" : "Text recognition options" }).hover();
    await expect(tooltip).toContainText(zh ? "已有文本层" : "text layer");
    await ocr.click(); await page.getByRole("option", { name: zh ? "强制 OCR" : "Force OCR", exact: true }).click();
    await expect(dialog.getByRole("button", { name: zh ? "保存" : "Save", exact: true })).toBeEnabled();
    await ocr.click(); await page.getByRole("option", { name: zh ? "提取已有文本" : "Extract existing text", exact: true }).click();
    await dialog.locator("summary").click(); await max.fill("600");
    await dialog.getByRole("button", { name: zh ? "保存" : "Save", exact: true }).click();
    await expect.poll(() => updates).toEqual([{ workspaceCode: "TRON_CORE", chunkConfig: { ...config, maxChars: 600 }, parsingTier: "standard", ocrMode: "Text" }]);
  });
}
