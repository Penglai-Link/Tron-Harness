import { expect, test } from "@playwright/test";
import { routeActiveProfileApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/workspace", { VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_CHAT_DATA_SOURCE: "mock", VITE_TRON_ADMIN_DATA_SOURCE: "mock", VITE_TRON_MODEL_DATA_SOURCE: "mock", VITE_TRON_AGENT_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

test("new chat opens its composer and reuses the existing draft", async ({ page }) => {
  const failures: string[] = [];
  page.on("pageerror", error => failures.push(error.message));
  await seedResourceUiShell(page);
  await routeActiveProfileApi(page);
  await page.goto(`${frontend!.baseUrl}/workspace`);
  const newChat = page.locator('[data-menu-code="CHAT_NEW_SESSION"]').getByRole("button", { name: /^New chat$/i });
  await expect(newChat).toBeEnabled();
  await newChat.click();
  await expect(page.locator('[data-theme-part="chat-input"]'), failures.join("\n")).toBeVisible({ timeout: 30000 });
  const url = page.url();
  await newChat.click();
  await expect(page).toHaveURL(url);
  await expect(page.locator('[data-theme-part="chat-input"]')).toBeVisible();
  expect(failures).toEqual([]);
});
