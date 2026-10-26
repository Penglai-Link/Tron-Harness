import { expect, test } from "@playwright/test";
import { routeActiveProfileApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend;
test.beforeAll(async () => {
  frontend = await startResourceUiFrontend("/settings/agents", {
    VITE_TRON_ADMIN_DATA_SOURCE: "mock", VITE_TRON_AGENT_DATA_SOURCE: "mock",
    VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_API_BASE_URL: "",
  });
});
test.afterAll(async () => stopResourceUiFrontend(frontend));

test("agent settings edit internal definitions and manage system activation in mock mode", async ({ page }) => {
  await seedResourceUiShell(page);
  const unexpected: string[] = [];
  await page.route("**/api/v1/**", async route => {
    const path = new URL(route.request().url()).pathname;
    if (path.startsWith("/api/v1/system/skins") || path === "/api/v1/dudes" || path === "/api/v1/admin/workspaces/options" || path === "/api/v1/agents/registry/internal/main-agent/skills" || path === "/api/v1/knowledge-bases/agent-grants/main-agent" || path === "/api/v1/observability/browser-spans") {
      return route.fulfill({ json: { code: "SUCCESS", data: path.endsWith("/active") ? null : [] } });
    }
    unexpected.push(path);
    await route.abort();
  });
  await routeActiveProfileApi(page);
  await page.goto(`${frontend.baseUrl}/settings/agents`);
  const internal = page.getByRole("region", { name: "Internal Agents", exact: true });
  await expect(internal.getByRole("button", { name: "Edit Main Agent · Built-in agent", exact: true })).toBeVisible();
  await expect(internal.getByText("fullstack-agent", { exact: true })).toHaveCount(0);
  await internal.getByRole("button", { name: "Edit Main Agent · Built-in agent", exact: true }).click();
  let dialog = page.getByRole("dialog", { name: "Edit Main Agent", exact: true });
  const prompt = dialog.getByRole("textbox", { name: "Instructions", exact: true });
  const updated = `${await prompt.inputValue()}\nValidate workspace inheritance before closing tasks.`;
  await prompt.fill(updated);
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await internal.getByRole("button", { name: "Edit Main Agent · Built-in agent", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Edit Main Agent", exact: true }).getByRole("textbox", { name: "Instructions", exact: true })).toHaveValue(updated);
  await page.getByRole("dialog", { name: "Edit Main Agent", exact: true }).getByRole("button", { name: "Close", exact: true }).click();

  const system = page.getByRole("region", { name: "System Agents", exact: true });
  await system.getByRole("button", { name: "Scan", exact: true }).click();
  await expect(page.getByRole("dialog").filter({ hasText: "Scan complete. Found 1 available system agents" })).toBeVisible();
  await expect(system.getByRole("button", { name: "Manage OpenCode", exact: true })).toHaveCount(0);
  await system.getByRole("button", { name: "Manage Codex", exact: true }).click();
  dialog = page.getByRole("dialog").filter({ has: page.getByRole("heading", { name: "Codex", exact: true }) });
  const activation = dialog.getByRole("switch");
  await expect(activation).toBeChecked();
  await activation.click();
  await expect(activation).not.toBeChecked();
  await expect(page.getByRole("dialog").filter({ hasText: "Codex disabled" })).toBeVisible();
  await activation.click();
  await expect(activation).toBeChecked();
  await expect(page.getByRole("dialog").filter({ hasText: "Codex enabled" })).toBeVisible();
  await dialog.getByRole("button", { name: "Edit", exact: true }).click();
  dialog = page.getByRole("dialog").filter({ has: page.getByRole("heading", { name: "Edit system Agent", exact: true }) });
  await expect(dialog.getByText("Use the base Agent’s existing login and subscription", { exact: false })).toHaveCount(0);
  await dialog.getByRole("button", { name: "Fast mode", exact: true }).hover();
  await expect(page.getByRole("tooltip")).toContainText("consumes more usage");
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(unexpected).toEqual([]);
});
