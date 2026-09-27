import { expect, test } from "@playwright/test";
import { routeActiveProfileApi, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";
// The desktop Profile migration removed server-managed users, roles and API keys.
// Keep a browser regression for old deep links rather than test deleted CRUD pages.
let frontend: ResourceUiFrontend;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/workspace", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });
for (const path of ["/admin/users", "/admin/users/new", "/admin/users/detail?userCode=old", "/admin/roles", "/admin/roles/matrix", "/admin/open-apps", "/admin/open-apps/new"]) {
  test(`retired identity-management route ${path} shows the current shell without legacy APIs`, async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("tron.locale", "en-US"));
    await routeActiveProfileApi(page);
    const requests: string[] = [];
    page.on("request", request => { if (/\/api\/v1\/(auth|admin\/(users|roles|open-apps))/.test(request.url())) requests.push(request.url()); });
    await page.goto(`${frontend.baseUrl}${path}`);
    await expect(page.getByRole("link", { name: "Users", exact: true })).toHaveCount(0);
    await expect(page.getByRole("link", { name: "Roles", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Open user menu", exact: true })).toContainText("Resource Operator");
    expect(requests).toEqual([]);
  });
}
