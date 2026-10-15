import { expect, test } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.use({ locale: "zh-CN" });
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/profiles"); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

test("first login defaults to English on a Chinese system and preserves an explicit language choice", async ({ page }) => {
  await routeResourceUiShellApi(page);
  await page.addInitScript(() => {
    if (!sessionStorage.getItem("initial-locale-cleared")) {
      localStorage.removeItem("tron.locale");
      sessionStorage.setItem("initial-locale-cleared", "true");
    }
  });
  await page.route("**/api/v1/profiles", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: [] } }));
  await page.route("**/api/v1/bootstrap/status", route => route.fulfill({ json: {
    code: "SUCCESS", message: "ok", data: { hasProfiles: false, activeProfileId: null, activeProfile: null, avatarUrl: null, account: null },
  } }));
  await page.goto(`${frontend!.baseUrl}/profiles`);
  await expect(page.getByRole("heading", { name: "Select user", exact: true })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("lang", "en-US");
  await expect(page.locator('[data-camera-phase="login"]')).toBeAttached();
  const language = page.getByRole("button", { name: "Language", exact: true });
  await expect(language).toHaveText("English");
  await language.click();
  await expect(page.getByRole("heading", { name: "选择用户", exact: true })).toBeVisible();
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await expect(page.getByRole("heading", { name: "选择用户", exact: true })).toBeVisible();
});

test("first authorization keeps the black hole when returning-user animation is disabled", async ({ page }) => {
  await seedResourceUiShell(page, "zh-CN");
  await routeResourceUiShellApi(page);
  await page.addInitScript(() => localStorage.setItem("tron.app-entrance-animation", "false"));
  await page.route("**/api/v1/profiles", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: [] } }));
  await page.route("**/api/v1/bootstrap/status", route => route.fulfill({ json: {
    code: "SUCCESS", message: "ok", data: { hasProfiles: false, activeProfileId: null, activeProfile: null, avatarUrl: null, account: null },
  } }));
  await page.goto(`${frontend!.baseUrl}/profiles`);
  await expect(page.locator(".black-hole-scene")).toBeVisible();
  await expect(page.locator('[data-camera-phase="login"]')).toBeAttached();
  await expect(page.getByRole("button", { name: "授权新账号", exact: true })).toBeVisible();
  await expect(page.locator(".black-hole-scene")).toBeVisible();
  await expect(page.locator('[data-tron-entry-ready]')).toHaveCount(0);
  await expect(page).toHaveURL(/\/profiles$/);
});

test("initialized users skip entrance by default and can persistently enable or disable it in Appearance", async ({ page }) => {
  await seedResourceUiShell(page, "zh-CN");
  await routeResourceUiShellApi(page);
  await page.route("**/api/v1/dudes/main/awakening", route => route.fulfill({ json: {
    code: "SUCCESS", message: "ok", data: { dudeCode: "main-dude", name: "Neo", state: "Ready", started: true, deferred: false, canResume: true },
  } }));
  await page.addInitScript(() => {
    const frames = (window as Window & { entranceFrames: string[] }).entranceFrames = [];
    new MutationObserver(() => {
      if (document.querySelector(".black-hole-scene")) frames.push("black-hole");
      if (document.querySelector(".awakening-home-veil")) frames.push("arrival");
    }).observe(document, { childList: true, subtree: true });
  });
  const appUrl = (path: string) => `${frontend!.baseUrl}${path}`;
  await page.goto(appUrl("/profiles"));
  await expect(page).toHaveURL(/\/workspace$/);
  await expect(page.locator('[data-tron-entry-ready]')).toBeAttached();
  expect(await page.evaluate(() => (window as Window & { entranceFrames: string[] }).entranceFrames)).toEqual([]);

  await page.goto(appUrl("/profiles?mode=switch"));
  const particles = page.locator("canvas[aria-hidden='true']");
  await expect(particles).toBeVisible();
  await expect(page.locator(".black-hole-scene")).toHaveCount(0);
  // Check painted particles, rather than accepting an empty canvas element.
  await expect.poll(() => particles.evaluate((canvas: HTMLCanvasElement) => {
    const pixels = canvas.getContext("2d")!.getImageData(0, 0, canvas.width, canvas.height).data;
    return pixels.some((value, index) => index % 4 === 3 && value > 0);
  })).toBe(true);

  await page.goto(appUrl("/settings/appearance"));
  const toggle = page.getByRole("switch", { name: "进入 App 动画", exact: true });
  await expect(toggle).not.toBeChecked();
  await toggle.click();
  await page.reload();
  await expect(toggle).toBeChecked();
  await page.goto(appUrl("/profiles"));
  await expect(page.locator(".black-hole-scene")).toBeVisible();
  await expect(page.locator('[data-camera-phase="dive"]')).toBeAttached();
  await expect(page).toHaveURL(/\/workspace$/, { timeout: 10_000 });
  await expect(page.locator(".awakening-home-veil")).toBeAttached();

  await page.goto(appUrl("/settings/appearance"));
  await toggle.click();
  await page.reload();
  await expect(toggle).not.toBeChecked();
  await page.goto(appUrl("/profiles"));
  await expect(page).toHaveURL(/\/workspace$/);
  expect(await page.evaluate(() => (window as Window & { entranceFrames: string[] }).entranceFrames)).toEqual([]);
});
