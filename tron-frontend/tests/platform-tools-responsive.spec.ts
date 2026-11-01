import { expect, test } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend;
const domainCode = "harness_workspace_automation_with_a_very_long_identifier";

test.beforeAll(async () => {
  frontend = await startResourceUiFrontend("/settings/tools", { VITE_TRON_AGENT_DATA_SOURCE: "api" });
});

test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const width of [390, 760, 1200]) {
  test(`tool domains stay inside their columns at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await routeResourceUiShellApi(page);
    await page.route(/\/api\/v1\/tools(?:\/domains|\/categories|\/permissions)?(?:\?.*)?$/, async (route) => {
      const path = new URL(route.request().url()).pathname;
      const data = path.endsWith("/domains")
        ? [{ domainCode, name: "Workspace automation", description: "Manage workspace automation and execution.", toolCount: 1 }]
        : path.endsWith("/categories") ? [{ code: "tron.platform", name: "Platform tools", toolCount: 1 }]
          : path.endsWith("/permissions") ? { permissions: [] }
            : [{ toolCode: "automation.list", toolName: "automation_list", domainCode, name: "List automations", description: "List workspace automations.", operationKind: "Query", category: { code: "tron.platform", names: {} } }];
      await route.fulfill({ json: { code: "SUCCESS", message: "ok", data } });
    });
    await seedResourceUiShell(page, "zh-CN");
    await page.goto(`${frontend.baseUrl}/settings/tools`);
    const button = page.getByRole("button", { name: `${domainCode}: Workspace automation` });
    await expect(button).toBeVisible();
    const bounds = await button.evaluate((element) => {
      const content = element.querySelector("span")!.getBoundingClientRect();
      const cell = element.closest("td")!.getBoundingClientRect();
      return { contentLeft: content.left, contentRight: content.right, cellLeft: cell.left, cellRight: cell.right };
    });
    expect(bounds.contentLeft).toBeGreaterThanOrEqual(bounds.cellLeft);
    expect(bounds.contentRight).toBeLessThanOrEqual(bounds.cellRight);
    await button.click();
    await expect(page.getByText("List workspace automations.")).toBeVisible();
    await expect(page.getByRole("group", { name: /List automations/ })).toBeAttached();
    const tableContainer = page.locator('[data-slot="table-container"]').first();
    const scroll = await tableContainer.evaluate((element) => ({ width: element.clientWidth, scrollWidth: element.scrollWidth }));
    if (width === 390) expect(scroll.scrollWidth).toBeGreaterThan(scroll.width);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  });
}
