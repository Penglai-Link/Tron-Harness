import { expect, test } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => {
  frontend = await startResourceUiFrontend("/settings/memory", {
    VITE_TRON_DATA_SOURCE: "mock",
    VITE_TRON_MEMORY_DATA_SOURCE: "api",
    VITE_TRON_KNOWLEDGE_GRAPH_DATA_SOURCE: "api",
  });
});
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const locale of ["zh-CN", "en-US"] as const) {
  test(`Memory table and graph canvas remain visible without a workspace in ${locale}`, async ({ page }, info) => {
    const zh = locale === "zh-CN";
    await routeResourceUiShellApi(page);
    await seedResourceUiShell(page, locale);
    let hasWorkspace = false;
    let memoryRequests = 0;
    let graphRequests = 0;
    const envelope = (data: unknown) => ({ code: "SUCCESS", message: "ok", data });
    await page.route("**/api/v1/admin/workspaces/options**", route => route.fulfill({ json: envelope(hasWorkspace ? [{ workspaceCode: "TRON_CORE", name: "Tron Core", label: "Tron Core", status: "Active" }] : []) }));
    await page.route("**/api/v1/memory/**", async route => {
      const path = new URL(route.request().url()).pathname;
      if (path.includes("/workspaces/")) {
        memoryRequests += 1;
        expect(path).toContain("/workspaces/TRON_CORE/records");
      }
      await route.fulfill({ json: envelope([]) });
    });
    await page.route("**/api/v1/knowledge-graph**", async route => {
      graphRequests += 1;
      expect(new URL(route.request().url()).searchParams.get("workspace-code")).toBe("TRON_CORE");
      expect(new URL(route.request().url()).pathname).toContain("list-namespaces");
      await route.fulfill({ json: envelope([]) });
    });

    await page.goto(`${frontend!.baseUrl}/settings/memory`);
    await expect(page.getByRole("table")).toBeVisible();
    await expect(page.getByRole("columnheader")).toHaveCount(6);
    await expect(page.getByRole("button", { name: zh ? "新建" : "New", exact: true })).toBeDisabled();
    await expect(page.getByRole("button", { name: zh ? "刷新" : "Refresh", exact: true })).toBeDisabled();
    await expect(page.getByRole("combobox", { name: zh ? "空间" : "Workspace", exact: true })).toContainText(zh ? "请选择空间" : "Select a workspace");
    expect(memoryRequests).toBe(0);
    await page.screenshot({ path: info.outputPath(`memory-empty-${locale}.png`), fullPage: true });

    await page.goto(`${frontend!.baseUrl}/settings/knowledge-graph?mode=business`);
    const canvas = page.getByRole("region", { name: zh ? "知识图谱" : "Knowledge graph", exact: true });
    await expect(canvas).toBeVisible();
    await expect(canvas.getByText(zh ? "0 个节点" : "0 nodes", { exact: true })).toBeVisible();
    await expect(canvas.getByText(zh ? "0 条边" : "0 edges", { exact: true })).toBeVisible();
    expect((await canvas.boundingBox())!.height).toBeGreaterThan(200);
    await expect(page.getByRole("button", { name: zh ? "刷新图谱" : "Refresh Graph", exact: true })).toBeDisabled();
    expect(graphRequests).toBe(0);
    await page.screenshot({ path: info.outputPath(`graph-empty-${locale}.png`), fullPage: true });

    hasWorkspace = true;
    await page.goto(`${frontend!.baseUrl}/settings/memory?workspaceCode=deleted`);
    await expect(page.getByRole("table")).toBeVisible();
    const scope = page.getByRole("combobox", { name: zh ? "空间" : "Workspace", exact: true });
    await expect(scope).toBeEnabled();
    await expect(page.getByRole("button", { name: zh ? "新建" : "New", exact: true })).toBeDisabled();
    expect(memoryRequests).toBe(0);
    await scope.click();
    await page.getByRole("option", { name: "Tron Core", exact: true }).click();
    await expect(scope).toContainText("Tron Core");
    await expect(page.getByRole("button", { name: zh ? "新建" : "New", exact: true })).toBeEnabled();
    await expect.poll(() => memoryRequests).toBeGreaterThan(0);
    await expect(page.getByRole("table")).toBeVisible();

    await page.goto(`${frontend!.baseUrl}/settings/knowledge-graph?workspaceCode=TRON_CORE&mode=business`);
    await expect(canvas).toBeVisible();
    await expect.poll(() => graphRequests).toBeGreaterThan(0);
    await expect(canvas.getByText(zh ? "0 个节点" : "0 nodes", { exact: true })).toBeVisible();
  });
}
