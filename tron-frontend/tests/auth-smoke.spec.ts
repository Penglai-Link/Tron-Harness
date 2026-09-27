import { expect, test, type Page } from "@playwright/test";
import { startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

// The desktop profile flow replaced local passwords, OAuth, LDAP and JWT refresh.
// Exercise the current identity boundary rather than resurrecting removed routes.
let frontend: ResourceUiFrontend;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/profiles", { VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_API_BASE_URL: "" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });
function profile(id: string, name: string) { return { profileId: id, personAiUserId: `USER-${id}`, personAiUsername: name, status: "ACTIVE", createdAt: "2026-09-27T00:00:00Z", lastActivatedAt: null, avatarUrl: null }; }
async function profilesApi(page: Page, count = 0, active: string | null = null) {
  const state = { profiles: [profile("one", "Operator One"), profile("two", "Operator Two")].slice(0, count), active, activated: [] as string[], removed: [] as string[], failActivation: false, failRemoval: false, tokenExpired: false, requests: [] as string[] };
  await page.addInitScript(() => localStorage.setItem("tron.locale", "en-US"));
  await page.route("**/api/v1/**", async route => {
    const path = new URL(route.request().url()).pathname;
    state.requests.push(path);
    const ok = (data: unknown) => route.fulfill({ json: { code: "SUCCESS", message: "success", data } });
    const failed = (message: string, status = 500) => route.fulfill({ status, json: { code: "PROFILE_ERROR", message, data: {} } });
    if (path === "/api/v1/bootstrap/status") return ok({ hasProfiles: state.profiles.length > 0, activeProfileId: state.active, activeProfile: state.profiles.find(p => p.profileId === state.active) ?? null, avatarUrl: null, account: null });
    if (path === "/api/v1/profiles") return ok(state.profiles);
    if (path === "/api/v1/profiles/events") return route.fulfill({ status: 204 });
    if (path === "/api/v1/profiles/active/token") return state.tokenExpired ? failed("Authorization expired", 401) : ok({ token: "profile-token" });
    const match = path.match(/^\/api\/v1\/profiles\/(one|two)(\/activate)?$/);
    if (match?.[2]) {
      if (state.failActivation) return failed("Switch failed");
      state.active = match[1]; state.activated.push(match[1]);
      return ok(state.profiles.find(p => p.profileId === state.active));
    }
    if (match && route.request().method() === "DELETE") {
      if (state.failRemoval) return failed("Removal failed");
      state.removed.push(match[1]); state.profiles = state.profiles.filter(p => p.profileId !== match[1]);
      if (state.active === match[1]) state.active = null;
      return ok({});
    }
    return route.fulfill({ status: 404, json: { code: "NOT_FOUND", message: path, data: {} } });
  });
  return state;
}

test("protected routes require a selected profile and show no legacy password login", async ({ page }) => {
  const state = await profilesApi(page);
  await page.goto(`${frontend.baseUrl}/apps`);
  await expect(page).toHaveURL(/\/profiles$/);
  await expect(page.getByRole("heading", { name: "Select user" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Authorize new account" })).toBeDisabled();
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  expect(state.requests.some(path => path.startsWith("/api/v1/auth/"))).toBe(false);
});

test("agreement enables new-account authorization and exposes its full text", async ({ page }) => {
  await profilesApi(page);
  await page.goto(`${frontend.baseUrl}/profiles`);
  await page.getByRole("button", { name: "User Agreement", exact: true }).click();
  await expect(page.getByRole("dialog")).toContainText("model APIs");
  await page.getByRole("button", { name: "Got it", exact: true }).click();
  await expect(page.getByRole("checkbox")).toBeChecked();
  await expect(page.getByRole("button", { name: "Authorize new account" })).toBeEnabled();
});

test("switching profiles activates the selected identity before opening the workspace", async ({ page }) => {
  const state = await profilesApi(page, 2, "one");
  await page.goto(`${frontend.baseUrl}/profiles?mode=switch`);
  await page.getByText("Operator Two", { exact: true }).click();
  await page.getByRole("button", { name: "Start using", exact: true }).click();
  await expect(page).toHaveURL(/\/workspace$/);
  await expect(page.getByRole("button", { name: "Open user menu" })).toContainText("Operator Two");
  expect(state.activated).toEqual(["two"]);
});

test("one already-active account enters the workspace automatically", async ({ page }) => {
  const state = await profilesApi(page, 1, "one");
  await page.goto(`${frontend.baseUrl}/profiles`);
  await expect(page).toHaveURL(/\/workspace$/);
  await expect(page.getByRole("button", { name: "Open user menu" })).toContainText("Operator One");
  expect(state.activated).toEqual([]);
});

test("failed activation stays on selection and reports the service error", async ({ page }) => {
  const state = await profilesApi(page, 2, "one"); state.failActivation = true;
  await page.goto(`${frontend.baseUrl}/profiles?mode=switch`);
  await page.getByText("Operator Two", { exact: true }).click();
  await page.getByRole("button", { name: "Start using", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Switch failed");
  expect(state.active).toBe("one");
});

test("expired authorization enters reauthorization after profile activation", async ({ page }) => {
  const state = await profilesApi(page, 2, "one"); state.tokenExpired = true;
  await page.goto(`${frontend.baseUrl}/profiles?mode=switch`);
  await page.getByRole("button", { name: "Start using", exact: true }).click();
  await expect(page).toHaveURL(/\/profiles\?mode=reauth$/);
  await expect(page.getByRole("button", { name: "Sign in again", exact: true })).toBeVisible();
});

test("account removal requires confirmation and cancelling preserves the profile", async ({ page }) => {
  const state = await profilesApi(page, 2, "one");
  await page.goto(`${frontend.baseUrl}/profiles?mode=switch`);
  await page.getByRole("button", { name: "Delete user", exact: true }).first().click();
  await expect(page.getByRole("dialog")).toContainText("Tron data will not be deleted");
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(state.removed).toEqual([]);
  await page.getByRole("button", { name: "Delete user", exact: true }).last().click();
  await page.getByRole("button", { name: "Confirm deletion", exact: true }).click();
  await expect(page.getByText("Operator Two", { exact: true })).toHaveCount(0);
  expect(state.removed).toEqual(["two"]);
});

test("failed deletion keeps the identity and reports the error in the dialog", async ({ page }) => {
  const state = await profilesApi(page, 2, "one"); state.failRemoval = true;
  await page.goto(`${frontend.baseUrl}/profiles?mode=switch`);
  await page.getByRole("button", { name: "Delete user", exact: true }).last().click();
  await page.getByRole("button", { name: "Confirm deletion", exact: true }).click();
  await expect(page.getByRole("dialog").getByRole("alert")).toContainText("Removal failed");
  expect(state.profiles).toHaveLength(2);
});
