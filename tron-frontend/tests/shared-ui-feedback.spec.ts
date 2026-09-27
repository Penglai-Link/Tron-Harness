import { expect, test } from "@playwright/test";
import { routeActiveProfileApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";
let frontend: ResourceUiFrontend;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/tasks", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });
test.beforeEach(async ({ page }) => { await seedResourceUiShell(page); await routeActiveProfileApi(page); await page.goto(`${frontend.baseUrl}/tasks`); });
test("successful refresh shows shared feedback within the viewport", async ({ page }) => {
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  const toast = page.getByRole("dialog").filter({ hasText: "Task board refreshed" });
  await expect(toast).toBeVisible();
  const box = await toast.boundingBox(); const viewport = page.viewportSize()!;
  expect(box).not.toBeNull(); expect(box!.x).toBeGreaterThanOrEqual(0); expect(box!.x + box!.width).toBeLessThanOrEqual(viewport.width); expect(box!.y + box!.height).toBeLessThanOrEqual(viewport.height);
});
test("shared feedback dismisses automatically and repeated actions create fresh feedback", async ({ page }) => {
  const toast = page.getByRole("dialog").filter({ hasText: "Task board refreshed" });
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(toast).toBeVisible(); await expect(toast).toHaveCount(0, { timeout: 10000 });
  await page.getByRole("button", { name: "Refresh", exact: true }).click(); await expect(toast).toBeVisible();
});
