import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/knowledge-base/kb-batch", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const locale of ["en-US", "zh-CN"] as const) {
  test(`Batch upload continues after failure and keeps parsing quality separate in ${locale}`, async ({ page }, testInfo) => {
    const zh = locale === "zh-CN";
    await page.setViewportSize({ width: 1075, height: 984 });
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    await page.route("**/api/v1/chats/session-groups?*", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: [] } }));
    const config = { mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2 };
    const uploaded: string[] = [];
    const requests: string[] = [];
    let parsed = false;
    let status = "QUEUED";
    let stage = "Parsing";
    await page.route("**/api/v1/knowledge-bases**", async route => {
      const request = route.request();
      const path = new URL(request.url()).pathname;
      const document = (fileName: string) => ({
        id: fileName, name: fileName, fileName, resourceCode: `res-${fileName}`, resourceRevision: 1,
        status: parsed ? "READY" : status, stage: parsed ? "Publishing" : stage, error: null,
        chunkCount: parsed ? 3 : 0, indexRevision: parsed ? 1 : 0, chunkConfig: null, parsingTier: null,
        parsingArtifact: parsed ? { effectiveTier: "standard", parserVersion: "4.0.10", resourceCode: `cache-${fileName}`, imageResourceCodes: [] } : null,
      });
      let data: unknown = { id: "kb-batch", name: "Guidelines", workspaceCode: "TRON_CORE", documents: uploaded.map(document), documentCount: uploaded.length, agentCodes: [], defaultChunkConfig: config };
      if (path.endsWith("/parsing-capabilities")) {
        data = { defaultTier: "standard", availableTiers: ["flash", "basic", "standard", "advanced"], parserVersion: "4.0.10", officeTier: "flash" };
      } else if (request.method() === "POST" && path.endsWith("/documents")) {
        expect(request.headers()["accept-language"]).toBe(locale);
        const body = request.postDataBuffer()!.toString();
        expect(body).toContain('name="workspaceCode"\r\n\r\nTRON_CORE');
        const name = /filename="([^"]+)"/.exec(body)![1];
        requests.push(name);
        if (name === "failed.pdf") {
          return route.fulfill({ status: 400, json: { code: "INVALID_ARGUMENT", message: "Upload refused", data: null } });
        }
        uploaded.push(name); data = document(name);
      }
      await route.fulfill({ json: { code: "SUCCESS", message: "ok", data } });
    });
    await page.goto(`${frontend!.baseUrl}/knowledge-base/kb-batch`);
    const input = page.locator('input[type="file"]');
    await expect(input).toHaveAttribute("multiple", "");
    await expect(input).toBeEnabled();
    await input.setInputFiles([
      { name: "first.pdf", mimeType: "application/pdf", buffer: Buffer.from("synthetic first PDF") },
      { name: "failed.pdf", mimeType: "application/pdf", buffer: Buffer.from("synthetic rejected PDF") },
      { name: "last.pdf", mimeType: "application/pdf", buffer: Buffer.from("synthetic last PDF") },
    ]);
    await expect.poll(() => requests).toEqual(["first.pdf", "failed.pdf", "last.pdf"]);
    await expect(page.getByText(zh ? "已上传 2 个文件。" : "Uploaded 2 files.", { exact: true })).toBeVisible();
    await expect(page.getByText(/failed\.pdf:.*Upload refused/)).toBeVisible();
    await expect(page.getByRole("columnheader", { name: zh ? "解析质量" : "Parsing quality", exact: true })).toBeVisible();
    await expect(page.getByRole("columnheader", { name: zh ? "切分方式" : "Splitting", exact: true })).toBeVisible();
    for (const fileName of ["first.pdf", "last.pdf"]) {
      const row = page.getByRole("row").filter({ hasText: fileName });
      await expect(row.getByRole("cell", { name: zh ? "标准" : "Standard", exact: true })).toBeVisible();
      await expect(row.getByRole("cell", { name: zh ? "自动" : "Automatic", exact: true })).toBeVisible();
      await expect(row.getByRole("cell", { name: zh ? "等待文档解析" : "Waiting for document parsing", exact: true })).toBeVisible();
    }
    // The status column combines queue state and live stage into a single label.
    for (const step of [
      { status: "PARSING", stage: "Parsing", label: zh ? "文档解析中" : "Parsing document" },
      { status: "CHUNKING", stage: "Chunking", label: zh ? "分块中" : "Chunking document" },
      { status: "QUEUED", stage: "Embedding", label: zh ? "等待向量化" : "Waiting for vector generation" },
      { status: "EMBEDDING", stage: "Embedding", label: zh ? "向量化中" : "Generating vectors" },
      { status: "PUBLISHING", stage: "Publishing", label: zh ? "索引发布中" : "Publishing index" },
      { status: "FAILED", stage: "Publishing", label: zh ? "失败" : "Failed" },
      { status: "CANCELLED", stage: "Publishing", label: zh ? "已取消" : "Cancelled" },
    ]) {
      status = step.status; stage = step.stage;
      await page.getByRole("button", { name: zh ? "刷新" : "Refresh", exact: true }).click();
      const cell = page.getByRole("row").filter({ hasText: "first.pdf" }).getByRole("cell", { name: step.label, exact: true });
      await expect(cell).toBeVisible();
      await expect(cell.locator("p")).toHaveCount(1);
      await expect(cell.locator('[data-slot="badge"]')).toHaveCount(0);
    }
    parsed = true;
    await page.getByRole("button", { name: zh ? "刷新" : "Refresh", exact: true }).click();
    const first = page.getByRole("row").filter({ hasText: "first.pdf" });
    await expect(first.getByText(zh ? "可用" : "Available", { exact: true })).toBeVisible();
    await expect(first.getByRole("cell", { name: zh ? "可用" : "Available", exact: true }).locator("p")).toHaveCount(1);
    await expect(first.getByRole("cell", { name: zh ? "标准" : "Standard", exact: true })).toBeVisible();
    await expect(first.getByRole("cell", { name: zh ? "自动" : "Automatic", exact: true })).toBeVisible();
    await page.screenshot({ animations: "disabled", path: testInfo.outputPath(`batch-upload-${locale}.png`) });
    // Clearing the native input lets the same selection be uploaded again.
    await input.setInputFiles({ name: "first.pdf", mimeType: "application/pdf", buffer: Buffer.from("synthetic first PDF") });
    await expect.poll(() => requests.length).toBe(4);
    await expect(page.getByText(zh ? "已上传 1 个文件。" : "Uploaded 1 file.", { exact: true })).toBeVisible();
  });
}
