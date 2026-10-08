import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/knowledge-base", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const locale of ["en-US", "zh-CN"] as const) {
  test(`Catalog counts, row navigation and guarded deletion in ${locale}`, async ({ page }, testInfo) => {
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    await page.route("**/api/v1/chats/session-groups?*", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: [] } }));
    let items = [
      { id: "kb-full", name: "Full library", description: "Library description", workspaceCode: "TRON_CORE", documentCount: 1, authorizedAgentCount: 2 },
      { id: "kb-empty", name: "Empty library", description: "Empty description", workspaceCode: "TRON_CORE", documentCount: 0, authorizedAgentCount: 3 },
    ];
    const deleted: string[] = [];
    await page.route("**/api/v1/knowledge-bases**", async route => {
      const url = new URL(route.request().url());
      expect(url.searchParams.get("workspaceCode")).toBe("TRON_CORE");
      let data: unknown = items;
      if (route.request().method() === "DELETE") {
        expect(url.pathname).toBe("/api/v1/knowledge-bases/kb-empty");
        expect(route.request().headers()["accept-language"]).toBe(locale);
        deleted.push(url.pathname); items = items.filter(item => item.id !== "kb-empty"); data = {};
      } else if (url.pathname.endsWith("/kb-full")) {
        data = { ...items[0], documents: [], agentCodes: ["a", "b"], defaultChunkConfig: { mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2 } };
      }
      await route.fulfill({ json: { code: "0", message: "ok", data } });
    });
    await page.goto(`${frontend!.baseUrl}/knowledge-base`);
    await expect(page.locator("main details")).toHaveCount(0);
    await expect(page.getByRole("button", { name: locale === "en-US" ? "RAG administration" : "RAG 管理", exact: true })).toHaveCount(0);
    const full = page.getByRole("row").filter({ hasText: "Full library" });
    const empty = page.getByRole("row").filter({ hasText: "Empty library" });
    await expect(full.locator("td").nth(3)).toHaveText("2");
    await expect(empty.locator("td").nth(3)).toHaveText("3");
    await full.getByRole("button").click();
    await expect(page).toHaveURL(/\/knowledge-base$/);
    const remove = page.getByRole("menuitem", { name: locale === "en-US" ? "Delete" : "删除", exact: true });
    await expect(remove).toHaveAttribute("aria-disabled", "true");
    await remove.locator("..").hover();
    await expect(page.getByRole("tooltip")).toContainText(locale === "en-US" ? "Delete all resources" : "请先删除知识库内的所有资源");
    await page.keyboard.press("Escape");
    await empty.getByRole("button").click(); await remove.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toContainText("Empty library");
    expect(deleted).toHaveLength(0);
    await dialog.getByRole("button", { name: locale === "en-US" ? "Cancel" : "取消", exact: true }).click();
    expect(deleted).toHaveLength(0);
    await empty.getByRole("button").click(); await remove.click();
    await dialog.getByRole("button", { name: locale === "en-US" ? "Delete" : "删除", exact: true }).click();
    await expect(empty).toHaveCount(0); expect(deleted).toHaveLength(1);
    await page.screenshot({ animations: "disabled", path: testInfo.outputPath(`knowledge-catalog-${locale}.png`) });
    await full.locator("td").nth(1).click();
    await expect(page).toHaveURL(/\/knowledge-base\/kb-full$/);
    await page.goto(`${frontend!.baseUrl}/knowledge-base`);
    await full.focus(); await full.press("Enter");
    await expect(page).toHaveURL(/\/knowledge-base\/kb-full$/);
  });
}
