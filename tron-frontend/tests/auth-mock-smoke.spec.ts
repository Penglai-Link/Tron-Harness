import { expect, test } from "@playwright/test";
import { routeActiveProfileApi, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";
let frontend: ResourceUiFrontend;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/apps", { VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_API_BASE_URL: "" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });
test("product mock mode uses profile identity without legacy auth requests", async ({ page }) => {
  const legacyRequests: string[] = [];
  await page.route("**/api/v1/auth/**", async route => { legacyRequests.push(route.request().url()); await route.abort(); });
  await routeActiveProfileApi(page);
  await page.goto(`${frontend.baseUrl}/apps`);
  await expect(page.getByRole("navigation", { name: "Tron primary navigation" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Open user menu" })).toContainText("Resource Operator");
  expect(legacyRequests).toEqual([]);
});
