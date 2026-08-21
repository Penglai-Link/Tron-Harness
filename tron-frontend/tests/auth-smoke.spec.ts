import {
  expect,
  test,
  type Page,
  type Route,
} from "@playwright/test";
import {
  spawnManagedProcess,
  stopManagedProcess,
  type ManagedProcess,
} from "./support/managed-process";
import { createServer } from "node:net";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

test.describe.configure({ timeout: 45_000 });

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
let frontendProcess: ManagedProcess | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

const authUser = {
  authMethod: "internal",
  authenticated: true,
  provider: null,
  user: {
    displayName: "Auth Operator",
    email: "auth.operator@tron.ai",
    roleCodes: ["ADMIN", "DEVELOPER"],
    spaceCodes: ["PLATFORM", "DATA"],
    status: "Active",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
  },
};

const oauthUser = {
  ...authUser,
  authMethod: "oauth2",
  provider: {
    code: "github",
    name: "GitHub",
  },
};

const ldapUser = {
  ...authUser,
  authMethod: "ldap",
  provider: {
    code: "ldap",
    name: "LDAP",
  },
};

const unassignedSpaceUser = {
  ...authUser,
  user: {
    ...authUser.user,
    displayName: "Unassigned Operator",
    email: "unassigned.operator@tron.ai",
    roleCodes: ["VIEWER"],
    spaceCodes: [],
    userCode: "USR-AUTH-0002",
    username: "unassigned.operator",
  },
};

const nonAdminUser = {
  ...authUser,
  user: {
    ...authUser.user,
    displayName: "Developer Operator",
    email: "developer.operator@tron.ai",
    roleCodes: ["DEVELOPER"],
    spaceCodes: ["PLATFORM"],
    userCode: "USR-DEVELOPER-0001",
    username: "developer.operator",
  },
};

const unassignedPlatformAdminUser = {
  ...authUser,
  user: {
    ...authUser.user,
    displayName: "Bootstrap Admin",
    email: "admin@tron.ai",
    roleCodes: ["ADMIN"],
    spaceCodes: [],
    userCode: "USR-ADMIN",
    username: "admin",
  },
};

function delay(ms: number) {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-4000);
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();

    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        rejectPort(new Error("Failed to reserve a frontend test port."));
        return;
      }

      const { port } = address;
      server.close(() => resolvePort(port));
    });
  });
}

async function waitForFrontend(url: string) {
  const deadline = Date.now() + 30_000;
  let lastError: unknown;

  while (Date.now() < deadline) {
    if (frontendProcess?.exitCode !== null) {
      throw new Error(`Frontend dev server exited early.\n${frontendLogs}`);
    }

    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch (error) {
      lastError = error;
    }

    await delay(250);
  }

  throw new Error(`Frontend dev server did not become ready: ${String(lastError)}\n${frontendLogs}`);
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
}

function successEnvelope(data: unknown) {
  return {
    code: "SUCCESS",
    message: "ok",
    data,
  };
}

function unauthorizedEnvelope(code = "AUTH_UNAUTHORIZED", message = "authentication required") {
  return {
    code,
    message,
    data: {},
  };
}

async function routeAuthApi(
  page: Page,
  options: {
    expireFirstAdminUsersRequest?: boolean;
    ldapLoginSession?: typeof ldapUser;
    loginGrantExpiresIn?: number;
    loginSession?: typeof authUser;
    onOAuth2Authorize?: (url: string) => void;
    onRefreshGrantIssued?: (count: number) => void;
    refreshGrantExpiresIn?: number;
  } = {},
) {
  let currentSession: typeof authUser | typeof oauthUser | typeof ldapUser | null = null;
  let hasRefreshSession = false;
  let currentAccessToken = "";
  let nextAccessTokenId = 1;
  let refreshGrantCount = 0;
  let shouldExpireAdminUsersRequest = options.expireFirstAdminUsersRequest ?? false;
  let accountProfile = {
    avatarUrl: null as string | null,
    displayName: "Auth Operator",
    email: "auth.operator@tron.ai",
    firstName: "Auth",
    lastName: "Operator",
    locale: "en-US",
    passwordChangeAllowed: true,
    passwordResetRequired: false,
    phone: null as { countryCode: string; number: string } | null,
    roleCodes: ["ADMIN", "DEVELOPER"],
    sourceType: "Internal",
    spaceCodes: ["PLATFORM", "DATA"],
    status: "Active",
    timezone: "UTC",
    updatedAt: "2026-07-11T00:00:00Z",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
  };

  function issueGrant(session: typeof authUser | typeof oauthUser | typeof ldapUser, expiresIn: number) {
    currentSession = session;
    hasRefreshSession = true;
    currentAccessToken = `test-access-token-${nextAccessTokenId++}`;

    return successEnvelope({
      accessToken: currentAccessToken,
      tokenType: "Bearer",
      expiresIn,
      session,
    });
  }

  await page.route("**/api/v1/auth/providers", async (route) => {
    await fulfillJson(route, successEnvelope({
      providers: [
        { code: "internal", name: "Internal Account", type: "internal" },
        { code: "github", name: "GitHub", type: "oauth2" },
        { code: "ldap", name: "LDAP", type: "ldap" },
      ],
    }));
  });

  await page.route("**/api/v1/auth/session", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope(currentSession));
  });

  await page.route("**/api/v1/auth/login", async (route) => {
    const body = route.request().postDataJSON() as { login?: string; password?: string };
    if (body.login === "auth.operator" && body.password === "TronAuthPass123!") {
      await fulfillJson(route, issueGrant(options.loginSession ?? authUser, options.loginGrantExpiresIn ?? 900));
      return;
    }

    await fulfillJson(route, unauthorizedEnvelope("AUTH_INVALID_CREDENTIALS", "invalid credentials"), 401);
  });

  await page.route("**/api/v1/auth/login/ldap", async (route) => {
    const body = route.request().postDataJSON() as { login?: string; password?: string };
    if (body.login === "auth.operator" && body.password === "TronAuthPass123!") {
      await fulfillJson(route, issueGrant(options.ldapLoginSession ?? ldapUser, options.loginGrantExpiresIn ?? 900));
      return;
    }

    await fulfillJson(route, unauthorizedEnvelope("AUTH_INVALID_CREDENTIALS", "invalid credentials"), 401);
  });

  await page.route("**/api/v1/auth/logout", async (route) => {
    currentAccessToken = "";
    hasRefreshSession = false;
    currentSession = null;
    await fulfillJson(route, successEnvelope({ loggedOut: true }));
  });

  await page.route("**/api/v1/auth/refresh", async (route) => {
    if (!currentSession || !hasRefreshSession) {
      await fulfillJson(route, unauthorizedEnvelope("AUTH_REFRESH_INVALID", "invalid refresh session"), 401);
      return;
    }

    refreshGrantCount += 1;
    options.onRefreshGrantIssued?.(refreshGrantCount);
    await fulfillJson(route, issueGrant(currentSession, options.refreshGrantExpiresIn ?? 900));
  });

  await page.route("**/api/v1/auth/oauth2/authorize/github**", async (route) => {
    options.onOAuth2Authorize?.(route.request().url());
    currentSession = oauthUser;
    hasRefreshSession = true;
    currentAccessToken = "";
    const url = new URL(route.request().url());
    const redirect = url.searchParams.get("redirect") ?? "/apps";
    await route.fulfill({
      status: 302,
      headers: {
        location: `${frontendBaseUrl}/auth/callback?redirect=${encodeURIComponent(redirect)}`,
      },
      body: "",
    });
  });

  await page.route("**/api/v1/account/get-profile", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope(accountProfile));
  });

  await page.route("**/api/v1/account/update-profile", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    const body = route.request().postDataJSON() as Partial<typeof accountProfile>;
    accountProfile = {
      ...accountProfile,
      ...body,
      updatedAt: "2026-07-11T01:00:00Z",
    };
    currentSession = {
      ...currentSession,
      user: {
        ...currentSession.user,
        displayName: accountProfile.displayName,
        email: accountProfile.email,
      },
    };
    await fulfillJson(route, successEnvelope(accountProfile));
  });

  await page.route("**/api/v1/account/change-password", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope({
      changed: true,
      passwordResetRequired: false,
    }));
  });

  await page.route("**/api/v1/admin/users**", async (route) => {
    if (shouldExpireAdminUsersRequest) {
      shouldExpireAdminUsersRequest = false;
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope({
      current: 1,
      size: 20,
      total: 1,
      records: [
        {
          id: "9007199254740993",
          userCode: "USR-AUTH-0001",
          username: "auth.operator",
          firstName: "Auth",
          lastName: "Operator",
          displayName: "Auth Operator",
          email: "auth.operator@tron.ai",
          phone: null,
          avatarUrl: null,
          locale: "en-US",
          timezone: "UTC",
          roleCodes: ["ADMIN", "DEVELOPER"],
          spaceCodes: ["PLATFORM", "DATA"],
          status: "Active",
          passwordResetRequired: false,
          lastActiveAt: "2026-05-15T10:00:00.000Z",
          createdAt: "2026-05-15T09:00:00.000Z",
          updatedAt: "2026-05-15T10:00:00.000Z",
        },
      ],
    }));
  });

  await page.route("**/api/v1/admin/users/statistics/**", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope({ count: 1 }));
  });

  await page.route("**/api/v1/admin/workspaces/switcher", async (route) => {
    if (
      !currentSession
      || route.request().headers().authorization !== `Bearer ${currentAccessToken}`
    ) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    if (currentSession.user.spaceCodes.length === 0) {
      await fulfillJson(route, successEnvelope({
        primarySpaces: [],
        secondarySpaces: [],
      }));
      return;
    }

    await fulfillJson(route, successEnvelope({
      primarySpaces: [
        {
          workspaceCode: "PLATFORM",
          name: "Platform",
          description: "Platform workspace",
          childCount: 1,
          status: "Active",
        },
      ],
      secondarySpaces: [
        {
          workspaceCode: "PLATFORM",
          primaryWorkspaceCode: "PLATFORM",
          label: "Platform",
          name: "Platform",
          description: "Platform workspace",
          depth: 1,
          childCount: 1,
          effectiveUserCount: 1,
          status: "Active",
        },
        {
          workspaceCode: "DATA",
          primaryWorkspaceCode: "PLATFORM",
          label: "Platform / Data",
          name: "Data",
          description: "Data workspace",
          depth: 2,
          childCount: 0,
          effectiveUserCount: 1,
          status: "Active",
        },
      ],
    }));
  });
}

async function submitLogin(page: Page) {
  await page.getByRole("button", { name: /sign in/i }).click({ force: true });
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;

  frontendProcess = spawnManagedProcess(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_DATA_SOURCE: "api",
      },
      stdio: "pipe",
    },
  );

  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/login`);
});

test.afterAll(async () => {
  try {
    await stopManagedProcess(frontendProcess);
  } finally {
    frontendProcess = undefined;
  }
});

test("redirects anonymous users to login, signs in, reaches user management, and returns to login after logout", async ({ page }) => {
  await routeAuthApi(page);

  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fapps$/);

  const loginInput = page.getByLabel(/operator id|username|email/i);
  const passwordInput = page.getByLabel(/passkey|password/i);
  await expect(loginInput).toHaveValue("");
  await expect(loginInput).toHaveAttribute("placeholder", "Enter username or email");
  await expect(passwordInput).toHaveValue("");

  await loginInput.fill("auth.operator");
  await passwordInput.fill("TronAuthPass123!");
  await submitLogin(page);

  await expect(page).toHaveURL(/\/apps$/);
  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page).toHaveURL(/\/admin\/users$/);
  await page.reload();
  await expect(page).toHaveURL(/\/admin\/users$/);

  await expect(page.getByRole("heading", { name: /users/i }).first()).toBeVisible();
  await expect(page.getByRole("table")).toContainText("Auth Operator");
  const primaryNavigation = page.getByRole("navigation", { name: /tron primary navigation/i });
  await expect(primaryNavigation.getByRole("button", { name: /administration/i })).toBeVisible();
  await expect(primaryNavigation.getByRole("link", { name: /^Users$/i })).toBeVisible();
  const header = page.locator("header");
  await expect(header.getByText("auth.operator@tron.ai")).toHaveCount(0);
  await expect(header.getByRole("button", { name: /^log out$/i })).toHaveCount(0);

  await page.getByRole("button", { name: /open user menu/i }).click();
  await expect(header.getByText("Auth Operator")).toBeVisible();
  await expect(header.getByText("auth.operator@tron.ai")).toBeVisible();
  await header.getByRole("button", { name: /^log out$/i }).click();
  await expect(page).toHaveURL(/\/login/);

  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fadmin%2Fusers$/);

  await page.goto(`${frontendBaseUrl}/settings/models`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fsettings%2Fmodels$/);
});

test("opens personal center from the user summary and updates profile, password, and theme", async ({ page }) => {
  await routeAuthApi(page);

  await page.goto(`${frontendBaseUrl}/apps`);
  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await submitLogin(page);
  await expect(page).toHaveURL(/\/apps$/);

  await page.getByRole("button", { name: /open user menu/i }).click();
  await page.getByRole("link", { name: /personal center/i }).click();
  await expect(page).toHaveURL(/\/account$/);
  await expect(page.getByRole("heading", { name: "Personal Center" })).toBeVisible();

  await page.getByLabel("Display name").fill("Alex Morgan");
  await page.getByRole("button", { name: /^Update$/ }).click();
  await expect(page.getByText("Personal information updated.")).toBeVisible();

  await page.getByRole("button", { name: /open user menu/i }).click();
  await expect(page.locator("header").getByText("Alex Morgan")).toBeVisible();
  await page.keyboard.press("Escape");

  await page.getByLabel("Current password").fill("TronAuthPass123!");
  await page.getByLabel("New password", { exact: true }).fill("TronNewPass456!");
  await page.getByLabel("Confirm new password").fill("TronNewPass456!");
  await page.getByRole("button", { name: "Change password" }).click();
  await expect(page.getByText("Password changed.")).toBeVisible();

  await page.getByRole("button", { name: "Matrix Dark" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "matrix-dark");
});

test("shows workspace guidance after closing the final tab", async ({ page }) => {
  await routeAuthApi(page);

  await page.goto(`${frontendBaseUrl}/apps`);
  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await submitLogin(page);

  const tabList = page.getByRole("tablist");
  await expect(tabList).toBeVisible();
  await tabList.getByRole("button", { name: "Close App Management" }).click();

  const emptyWorkspace = page.getByTestId("tron-empty-tab-workspace");
  await expect(emptyWorkspace).toBeVisible();
  await expect(emptyWorkspace.getByRole("heading", { name: "No open tabs" })).toBeVisible();
  await expect(emptyWorkspace.getByText("Open a view")).toBeVisible();
});

test("blocks authenticated users without assigned spaces on an authorization request page", async ({ page }) => {
  await routeAuthApi(page, {
    loginSession: unassignedSpaceUser,
  });

  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fapps$/);

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await submitLogin(page);

  await expect(page).toHaveURL(/\/apps$/);
  await expect(page.getByRole("heading", { name: /workspace authorization required/i })).toBeVisible();
  await expect(page.getByText("Unassigned Operator")).toBeVisible();
  await expect(page.getByRole("navigation", { name: /tron primary navigation/i })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /open workspace selector/i })).toHaveCount(0);

  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page.getByRole("heading", { name: /workspace authorization required/i })).toBeVisible();
  await expect(page.getByRole("table")).toHaveCount(0);

  await page.getByRole("button", { name: /^log out$/i }).click({ force: true });
  await expect(page).toHaveURL(/\/login/);
});

test("allows platform admins to bypass the no-space authorization page", async ({ page }) => {
  await routeAuthApi(page, {
    loginSession: unassignedPlatformAdminUser,
  });

  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fapps$/);

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await submitLogin(page);

  await expect(page).toHaveURL(/\/apps$/);
  await expect(page.getByRole("heading", { name: /workspace authorization required/i })).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: /tron primary navigation/i })).toBeVisible();
  await expect(page.getByRole("button", { name: /open workspace selector/i })).toBeVisible();
});

test("hides administration navigation for non-admin users", async ({ page }) => {
  await routeAuthApi(page, {
    loginSession: nonAdminUser,
  });

  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fapps$/);

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await submitLogin(page);

  await expect(page).toHaveURL(/\/apps$/);
  const primaryNavigation = page.getByRole("navigation", { name: /tron primary navigation/i });
  await expect(primaryNavigation).toBeVisible();
  await expect(primaryNavigation.getByRole("link", { name: /^Apps$/i })).toBeVisible();
  await expect(primaryNavigation.getByRole("button", { name: /administration/i })).toHaveCount(0);
  await expect(primaryNavigation.getByRole("link", { name: /^Users$/i })).toHaveCount(0);
  await expect(primaryNavigation.getByRole("link", { name: /^Roles$/i })).toHaveCount(0);
  await expect(primaryNavigation.getByRole("link", { name: /^Audit Logs$/i })).toHaveCount(0);
  await expect(primaryNavigation.getByRole("link", { name: /^Workspace$/i })).toHaveCount(0);

  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page).toHaveURL(/\/apps$/);
  await expect(page.getByRole("status").filter({ hasText: /access restricted/i })).toBeVisible();
  await expect(page.getByRole("heading", { name: /users/i })).toHaveCount(0);
});

test("supports oauth2 entry and lands on the intended protected route", async ({ page }) => {
  const localhostBaseUrl = frontendBaseUrl.replace("127.0.0.1", "localhost");
  const frontendPort = new URL(frontendBaseUrl).port;
  let authorizeRequestUrl = "";

  await routeAuthApi(page, {
    onOAuth2Authorize(url) {
      authorizeRequestUrl = url;
    },
  });

  await page.goto(`${localhostBaseUrl}/login?redirect=%2Fadmin%2Fusers`);
  await page.getByRole("button", { name: /^github$/i }).click();

  await expect.poll(() => authorizeRequestUrl).toBe(
    `http://127.0.0.1:${frontendPort}/api/v1/auth/oauth2/authorize/github?redirect=%2Fadmin%2Fusers`,
  );
  await expect(page).toHaveURL(`http://127.0.0.1:${frontendPort}/admin/users`);
  await expect(page.getByRole("table")).toContainText("Auth Operator");
});

test("supports ldap entry below GitHub and lands on the intended protected route", async ({ page }) => {
  await routeAuthApi(page);

  await page.goto(`${frontendBaseUrl}/login?redirect=%2Fadmin%2Fusers`);
  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");

  const buttonTexts = await page.locator("button").evaluateAll((buttons) => (
    buttons
      .map((button) => button.textContent?.replace(/\s+/g, " ").trim() ?? "")
      .filter(Boolean)
  ));
  expect(buttonTexts.findIndex((text) => /^GitHub$/i.test(text))).toBeGreaterThanOrEqual(0);
  expect(buttonTexts.findIndex((text) => /^LDAP$/i.test(text))).toBeGreaterThanOrEqual(0);
  expect(buttonTexts.findIndex((text) => /^GitHub$/i.test(text))).toBeLessThan(
    buttonTexts.findIndex((text) => /^LDAP$/i.test(text)),
  );

  await page.getByRole("button", { name: /^ldap$/i }).click();

  await expect(page).toHaveURL(/\/admin\/users$/);
  await expect(page.getByRole("table")).toContainText("Auth Operator");
});

test("refreshes the access token before it expires while the operator stays idle", async ({ page }) => {
  let refreshGrantCount = 0;

  await routeAuthApi(page, {
    loginGrantExpiresIn: 2,
    onRefreshGrantIssued(count) {
      refreshGrantCount = count;
    },
  });

  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fapps$/);

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await page.getByRole("button", { name: /sign in/i }).click();

  await expect(page).toHaveURL(/\/apps$/);
  await expect.poll(() => refreshGrantCount, { timeout: 6_000 }).toBe(1);

  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page).toHaveURL(/\/admin\/users$/);
  await expect(page.getByRole("table")).toContainText("Auth Operator");
});

test("refreshes and retries a protected request after the first 401 instead of redirecting to login", async ({ page }) => {
  let refreshGrantCount = 0;

  await routeAuthApi(page, {
    expireFirstAdminUsersRequest: true,
    onRefreshGrantIssued(count) {
      refreshGrantCount = count;
    },
  });

  await page.goto(`${frontendBaseUrl}/apps`);
  await expect(page).toHaveURL(/\/login\?redirect=%2Fapps$/);

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await page.getByRole("button", { name: /sign in/i }).click();

  await expect(page).toHaveURL(/\/apps$/);
  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect.poll(() => refreshGrantCount > 0).toBe(true);
  await expect(page).toHaveURL(/\/admin\/users$/);
  await expect(page).not.toHaveURL(/\/login/);
  await expect(page.getByRole("table")).toContainText("Auth Operator");
});
