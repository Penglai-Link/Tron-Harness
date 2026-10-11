import { expect, test } from "@playwright/test";
import { routeActiveProfileApi, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/workspace", {
  VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_ADMIN_DATA_SOURCE: "api", VITE_TRON_CHAT_DATA_SOURCE: "api", VITE_TRON_PROJECT_DATA_SOURCE: "api", VITE_TRON_API_BASE_URL: "",
}); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

test("first login validates persisted workspace before requesting chat groups", async ({ page }) => {
  const space = (code: string) => ({ childCount: 0, name: "Default workspace", status: "Active", workspaceCode: code, description: "", depth: 1, primaryWorkspaceCode: code, effectiveUserCount: 1, label: "Default workspace" });
  await page.addInitScript((stale) => {
    localStorage.setItem("tron.locale", "en-US");
    localStorage.setItem("tron-app-shell", JSON.stringify({ version: 9, state: { selectedPrimarySpaceId: stale.workspaceCode, workspaceSwitcherSnapshot: { primarySpaces: [stale], secondarySpaces: [stale] } } }));
  }, space("WS-OLD"));
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  let switcherRequested = false;
  const workspaces: string[] = [];
  await page.route("**/api/v1/**", async (route) => {
    const url = new URL(route.request().url());
    const ok = (data: unknown) => route.fulfill({ json: { code: "SUCCESS", message: "ok", data } });
    if (url.pathname === "/api/v1/admin/workspaces/switcher") {
      switcherRequested = true;
      await gate;
      return ok({ primarySpaces: [space("WS-NEW")], secondarySpaces: [space("WS-NEW")] });
    }
    if (url.pathname === "/api/v1/chats/session-groups") {
      workspaces.push(url.searchParams.get("workspaceCode") ?? "");
    }
    return ok([]);
  });
  await routeActiveProfileApi(page);
  await page.goto(`${frontend.baseUrl}/workspace`);
  await expect.poll(() => switcherRequested).toBe(true);
  expect(workspaces).toEqual([]);
  release();
  await expect.poll(() => workspaces.length).toBeGreaterThan(0);
  expect(workspaces.every((code) => code === "WS-NEW")).toBe(true);
  await expect(page.getByText(/failed with 404/)).toHaveCount(0);
});
