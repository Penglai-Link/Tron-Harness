import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/settings/knowledge-base/kb-retry", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const locale of ["zh-CN", "en-US"] as const) {
  test(`Batch retry submits only selected documents and retains failures in ${locale}`, async ({ page }, info) => {
    const zh = locale === "zh-CN";
    await routeResourceUiShellApi(page);
    await seedResourceUiShell(page, locale);
    const config = { mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2 };
    const documents = [
      { id: "running", status: "PARSING" },
      { id: "queued", status: "QUEUED" },
      { id: "ready", status: "READY" },
      { id: "good", status: "FAILED" },
      { id: "http-error", status: "FAILED" },
      { id: "cancelled", status: "CANCELLED" },
      { id: "review", status: "REVIEW_REQUIRED" },
      { id: "untouched", status: "FAILED" },
    ].map(doc => ({ ...doc, fileName: `${doc.id}.pdf`, resourceCode: `res-${doc.id}`, resourceRevision: 1, error: null, chunkCount: 0, indexRevision: 0, chunkConfig: config, parsingTier: "standard" }));
    const requests: string[] = [];
    let reject = true;
    let releaseFirst!: () => void;
    const firstRequestGate = new Promise<void>(resolve => { releaseFirst = resolve; });
    await page.route("**/api/v1/knowledge-bases/**", async route => {
      const request = route.request();
      const path = new URL(request.url()).pathname;
      if (request.method() === "POST") {
        expect(path).toMatch(/^\/api\/v1\/knowledge-bases\/kb-retry\/documents\/[^/]+\/retry$/);
        const id = path.split("/").at(-2)!;
        requests.push(id);
        if (requests.length === 1) await firstRequestGate;
        if (id === "http-error" && reject) return route.fulfill({ status: 503, json: { code: "RETRY_FAILED", message: "Synthetic retry failure", data: null } });
        const doc = documents.find(doc => doc.id === id)!;
        if (id === "review" && reject) return route.fulfill({ json: { code: "SUCCESS", message: "ok", data: { ...doc, status: "FAILED", error: "Synthetic document failure" } } });
        doc.status = "QUEUED";
        return route.fulfill({ json: { code: "SUCCESS", message: "ok", data: doc } });
      }
      expect(path).toBe("/api/v1/knowledge-bases/kb-retry");
      return route.fulfill({ json: { code: "SUCCESS", message: "ok", data: {
        id: "kb-retry", name: "Guidelines", description: "", agentCodes: [], documentCount: documents.length, defaultChunkConfig: config, documents,
      } } });
    });
    await page.goto(`${frontend!.baseUrl}/settings/knowledge-base/kb-retry`);
    const button = page.getByRole("button", { name: zh ? "批量重试" : "Retry selected", exact: true });
    const checkbox = (id: string) => page.getByRole("checkbox", { name: zh ? `选择 ${id}.pdf` : `Select ${id}.pdf`, exact: true });
    const all = page.getByRole("checkbox", { name: zh ? "全选可重试文档" : "Select all retryable documents", exact: true });
    await expect(button).toBeDisabled();
    for (const id of ["running", "queued", "ready"]) await expect(checkbox(id)).toBeDisabled();
    await all.check();
    for (const id of ["good", "http-error", "cancelled", "review", "untouched"]) await expect(checkbox(id)).toBeChecked();
    for (const id of ["running", "queued", "ready"]) await expect(checkbox(id)).not.toBeChecked();
    await all.uncheck();
    await expect(button).toBeDisabled();
    for (const id of ["good", "http-error", "cancelled", "review"]) await checkbox(id).check();
    await expect(all).toHaveJSProperty("indeterminate", true);
    await expect(button).toBeEnabled();
    await page.screenshot({ path: info.outputPath(`batch-retry-selected-${locale}.png`), fullPage: true });
    await button.click();
    const pending = page.getByRole("button", { name: zh ? "提交中…" : "Submitting…", exact: true });
    await expect(pending).toBeDisabled();
    await expect(all).toBeDisabled();
    await expect(checkbox("http-error")).toBeDisabled();
    releaseFirst();
    await expect.poll(() => requests).toEqual(["good", "http-error", "cancelled", "review"]);
    await expect(button).toBeEnabled();
    await expect(page.getByText("Synthetic retry failure", { exact: false })).toBeVisible();
    await expect(page.getByText("Synthetic document failure", { exact: false })).toBeVisible();
    for (const id of ["good", "cancelled"]) {
      await expect(checkbox(id)).not.toBeChecked();
      await expect(checkbox(id)).toBeDisabled();
    }
    for (const id of ["http-error", "review"]) await expect(checkbox(id)).toBeChecked();
    await expect(checkbox("untouched")).not.toBeChecked();
    reject = false;
    await button.click();
    await expect.poll(() => requests).toEqual(["good", "http-error", "cancelled", "review", "http-error", "review"]);
    await expect(button).toBeDisabled();
    for (const id of ["http-error", "review"]) await expect(checkbox(id)).not.toBeChecked();
    expect(requests).not.toContain("untouched");
  });
}
