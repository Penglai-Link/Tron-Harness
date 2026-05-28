import {
  expect,
  test,
  type Page,
  type Route,
} from "@playwright/test";
import {
  spawn,
  type ChildProcessWithoutNullStreams,
} from "node:child_process";
import { readFileSync } from "node:fs";
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

const alexUserRecord = {
  id: "9007199254740993",
  userCode: "USER_ALEX_JOHNSON",
  sourceType: "Internal",
  username: "alex.johnson",
  firstName: "Alex",
  lastName: "Johnson",
  displayName: "Alex Johnson",
  email: "alex.johnson@example.com",
  phone: null,
  avatarUrl: null,
  locale: "en-US",
  timezone: "UTC",
  roleCodes: ["ADMIN", "DEVELOPER"],
  spaceCodes: ["PLATFORM", "DATA"],
  status: "Active",
  passwordResetRequired: false,
  lastActiveAt: null,
  createdAt: "2026-05-13T00:00:00.000Z",
  updatedAt: "2026-05-13T00:00:00.000Z",
  archivedAt: null,
};

type CapturedAdminRequest = {
  body: unknown;
  method: string;
  pathname: string;
  searchParams: Record<string, string>;
};

const backendTemporaryPassword = "TronTmp734103002743185409!";
const redExpectationTimeout = 2500;

function source(pathFromFrontendRoot: string): string {
  return readFileSync(
    resolve(frontendRoot, pathFromFrontendRoot),
    "utf8",
  );
}

function delay(ms: number) {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

function formatUiTimestamp(value: string | null) {
  if (!value) return "Never";
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-4000);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function makeUserRecord(overrides: Partial<typeof alexUserRecord>) {
  return {
    ...alexUserRecord,
    ...overrides,
    roleCodes: overrides.roleCodes ?? [...alexUserRecord.roleCodes],
    spaceCodes: overrides.spaceCodes ?? [...alexUserRecord.spaceCodes],
  };
}

function captureAdminRequest(route: Route): CapturedAdminRequest {
  const request = route.request();
  const url = new URL(request.url());
  const rawBody = request.postData();
  let body: unknown;

  if (rawBody) {
    try {
      body = JSON.parse(rawBody) as unknown;
    } catch {
      body = rawBody;
    }
  }

  return {
    body,
    method: request.method(),
    pathname: url.pathname,
    searchParams: Object.fromEntries(url.searchParams.entries()),
  };
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

function pagedEnvelope(records: unknown[], current = 1, size = 20, total = records.length) {
  return successEnvelope({
    current,
    size,
    total,
    records,
  });
}

async function routeAuthenticatedSession(page: Page) {
  const session = {
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
  let currentAccessToken = "user-management-smoke-access-token";
  let hasRefreshSession = true;

  function issueGrant() {
    currentAccessToken = "user-management-smoke-access-token";
    hasRefreshSession = true;
    return successEnvelope({
      accessToken: currentAccessToken,
      tokenType: "Bearer",
      expiresIn: 900,
      session,
    });
  }

  await page.route("**/api/v1/auth/providers", async (route) => {
    await fulfillJson(route, successEnvelope({
      providers: [
        { code: "internal", name: "Internal Account", type: "internal" },
        { code: "github", name: "GitHub", type: "oauth2" },
      ],
    }));
  });

  await page.route("**/api/v1/auth/session", async (route) => {
    if (route.request().headers().authorization !== `Bearer ${currentAccessToken}`) {
      await fulfillJson(route, unauthorizedEnvelope(), 401);
      return;
    }

    await fulfillJson(route, successEnvelope(session));
  });

  await page.route("**/api/v1/auth/login", async (route) => {
    const body = route.request().postDataJSON() as { login?: string; password?: string };
    if (body.login === "auth.operator" && body.password === "TronAuthPass123!") {
      await fulfillJson(route, issueGrant());
      return;
    }

    await fulfillJson(route, unauthorizedEnvelope("AUTH_INVALID_CREDENTIALS", "invalid credentials"), 401);
  });

  await page.route("**/api/v1/auth/refresh", async (route) => {
    if (!hasRefreshSession) {
      await fulfillJson(route, unauthorizedEnvelope("AUTH_REFRESH_INVALID", "invalid refresh session"), 401);
      return;
    }

    await fulfillJson(route, issueGrant());
  });

  await page.route("**/api/v1/auth/logout", async (route) => {
    currentAccessToken = "";
    hasRefreshSession = false;
    await fulfillJson(route, successEnvelope({ loggedOut: true }));
  });

  await page.route("**/api/v1/admin/workspaces/switcher", async (route) => {
    await fulfillJson(route, successEnvelope({
      primarySpaces: [
        {
          childCount: 0,
          description: "Primary workspace for authenticated admin smoke tests.",
          name: "Primary Space",
          status: "Active",
          workspaceCode: "PLATFORM",
        },
      ],
      secondarySpaces: [
        {
          childCount: 0,
          depth: 1,
          description: "Root workspace context for authenticated admin smoke tests.",
          effectiveUserCount: 1,
          label: "Primary Space",
          name: "Primary Space",
          primaryWorkspaceCode: "PLATFORM",
          status: "Active",
          workspaceCode: "PLATFORM",
        },
      ],
    }));
  });
}

async function signInToProtectedRoute(page: Page, targetPath: string) {
  await page.goto(`${frontendBaseUrl}${targetPath}`);
  await expect(page).toHaveURL(`${frontendBaseUrl}${targetPath}`);
}

function sharedToastViewport(page: Page) {
  return page.locator('[aria-label="Notifications"]').first();
}

function sharedToastMessage(page: Page, message: string | RegExp) {
  return sharedToastViewport(page).getByRole("status").filter({ hasText: message }).first();
}

function userDirectoryLink(page: Page, displayName: string) {
  return page.getByRole("table").getByRole("link", { name: displayName, exact: true });
}

async function expectSharedTopRightToast(page: Page, message: string | RegExp, description: string) {
  const toast = sharedToastMessage(page, message);
  await expect(
    toast,
    `${description} should render through the shared top-right Toast viewport, not page-local status text.`,
  ).toBeVisible({ timeout: redExpectationTimeout });

  const viewportBox = await sharedToastViewport(page).boundingBox();
  const viewportSize = page.viewportSize();
  expect.soft(viewportBox, `${description} should have a measurable shared toast viewport.`).not.toBeNull();
  if (viewportBox && viewportSize) {
    expect.soft(viewportBox.x, `${description} toast viewport should be aligned on the right half of the viewport.`).toBeGreaterThan(viewportSize.width / 2);
    expect.soft(viewportBox.y, `${description} toast viewport should be near the top of the viewport.`).toBeLessThan(80);
  }
}

async function expectCapturedAdminRequest(
  requests: CapturedAdminRequest[],
  description: string,
  predicate: (request: CapturedAdminRequest) => boolean,
  options: { after?: number; soft?: boolean } = {},
) {
  const assertion = options.soft ? expect.configure({ soft: true }) : expect;

  await assertion.poll(
    () => requests.slice(options.after ?? 0).some(predicate),
    {
      message: `Expected admin user API request: ${description}`,
      timeout: 2500,
    },
  ).toBe(true);
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

async function routeAdminUsers(page: Page, records: unknown[]) {
  await page.route("**/api/v1/admin/users**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());

    if (request.method() === "GET" && /^\/api\/v1\/admin\/users\/[^/]+$/.test(url.pathname)) {
      const userCode = decodeURIComponent(url.pathname.split("/").at(-1) ?? "");
      const record = records.find((candidate) => isRecord(candidate) && candidate.userCode === userCode);
      await fulfillJson(
        route,
        record
          ? successEnvelope(record)
          : { code: "ADMIN_USER_NOT_FOUND", message: "user not found", data: {} },
        record ? 200 : 404,
      );
      return;
    }

    await fulfillJson(route, pagedEnvelope(records));
  });
}

async function routeDirectoryQueryApi(page: Page, requests: CapturedAdminRequest[]) {
  const initialRecords = Array.from({ length: 16 }, (_, index) => {
    const ordinal = String(index + 1).padStart(2, "0");
    return makeUserRecord({
      id: `7341030000000000${ordinal}`,
      userCode: `USER_DIRECTORY_${ordinal}`,
      username: `directory.${ordinal}`,
      firstName: "Directory",
      lastName: `User ${ordinal}`,
      displayName: `Directory User ${ordinal}`,
      email: `directory.${ordinal}@tron.ai`,
      roleCodes: ["VIEWER"],
      spaceCodes: ["PLATFORM"],
      status: "Active",
    });
  });
  const pageTwoRecord = makeUserRecord({
    id: "734103000000009999",
    userCode: "USER_PAGE_TWO_FROM_API",
    username: "page.two",
    firstName: "Page",
    lastName: "Two",
    displayName: "Page Two From API",
    email: "page.two@tron.ai",
    roleCodes: ["DEVELOPER"],
    spaceCodes: ["DATA"],
    status: "Active",
  });
  const searchRecord = makeUserRecord({
    id: "734103000000008888",
    userCode: "USER_SERVER_SEARCH_MATCH",
    username: "server.search",
    firstName: "Server",
    lastName: "Search",
    displayName: "Server Search Match",
    email: "server.search@tron.ai",
    roleCodes: ["ADMIN"],
    spaceCodes: ["PLATFORM"],
    status: "Active",
  });
  const archivedRecord = makeUserRecord({
    id: "734103000000007777",
    userCode: "USER_ARCHIVED_SERVER_RESULT",
    username: "archived.server",
    firstName: "Archived",
    lastName: "Server",
    displayName: "Archived Server Result",
    email: "archived.server@tron.ai",
    roleCodes: ["VIEWER"],
    spaceCodes: ["DATA"],
    status: "Archived",
    archivedAt: "2026-05-13T01:00:00.000Z",
  });

  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method !== "GET" || captured.pathname !== "/api/v1/admin/users") {
      await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
      return;
    }

    const current = Number(captured.searchParams.current ?? 1);
    const size = Number(captured.searchParams.size ?? 15);
    const records = captured.searchParams.search === "server-search" && captured.searchParams.status === "Archived"
      ? [archivedRecord]
      : captured.searchParams.search === "server-search"
        ? [searchRecord]
        : current === 2
          ? [pageTwoRecord]
          : initialRecords;

    await fulfillJson(route, pagedEnvelope(records, current, size, current === 2 ? 30 : records.length));
  });
}

async function routeAllStatusIncludeArchivedApi(page: Page, requests: CapturedAdminRequest[]) {
  const activeRecord = makeUserRecord({
    id: "734103000000004201",
    userCode: "USER_ALL_STATUS_ACTIVE",
    username: "all.status.active",
    firstName: "All",
    lastName: "Active",
    displayName: "All Status Active",
    email: "all.status.active@tron.ai",
    roleCodes: ["ADMIN"],
    spaceCodes: ["PLATFORM"],
    status: "Active",
  });
  const archivedRecord = makeUserRecord({
    id: "734103000000004202",
    userCode: "USER_ALL_STATUS_ARCHIVED",
    username: "all.status.archived",
    firstName: "All",
    lastName: "Archived",
    displayName: "All Status Archived",
    email: "all.status.archived@tron.ai",
    roleCodes: ["VIEWER"],
    spaceCodes: ["DATA"],
    status: "Archived",
    archivedAt: "2026-05-13T04:20:00.000Z",
  });

  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method !== "GET" || captured.pathname !== "/api/v1/admin/users") {
      await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
      return;
    }

    const current = Number(captured.searchParams.current ?? 1);
    const size = Number(captured.searchParams.size ?? 15);
    const records = captured.searchParams.includeArchived === "true"
      ? [activeRecord, archivedRecord]
      : [activeRecord];

    await fulfillJson(route, pagedEnvelope(records, current, size, records.length));
  });
}

async function routeCreateUserApi(
  page: Page,
  requests: CapturedAdminRequest[],
  options: { createDelayMs?: number } = {},
) {
  const createdUser = makeUserRecord({
    id: "734103002743185408",
    userCode: "USER_MAYA_SINGH",
    username: "maya.singh",
    firstName: "Maya",
    lastName: "Singh",
    displayName: "Maya Singh",
    email: "maya@tron.ai",
    phone: { countryCode: "+1", number: "5550101" },
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: [],
    spaceCodes: [],
    status: "Active",
    passwordResetRequired: true,
    createdAt: "2026-05-13T02:00:00.000Z",
    updatedAt: "2026-05-13T02:00:00.000Z",
  });
  let created = false;

  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users") {
      await fulfillJson(route, pagedEnvelope(created ? [alexUserRecord, createdUser] : [alexUserRecord]));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users") {
      created = true;
      await delay(options.createDelayMs ?? 0);
      await fulfillJson(route, successEnvelope(createdUser), 201);
      return;
    }

    await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
  });
}

async function routeConsistencyRefactorApi(page: Page, requests: CapturedAdminRequest[]) {
  const directoryUser = makeUserRecord({
    id: "734103002743185501",
    userCode: "USER_SAM_DIRECTORY",
    username: "sam.directory",
    firstName: "Sam",
    lastName: "Directory",
    displayName: "Sam Directory",
    email: "sam.directory@tron.ai",
    phone: { countryCode: "+1", number: "5550199" },
    roleCodes: ["DEVELOPER"],
    spaceCodes: ["PLATFORM", "DATA"],
    status: "Active",
  });
  const createdUser = makeUserRecord({
    id: "734103002743185408",
    userCode: "USER_MAYA_SINGH",
    username: "maya.singh",
    firstName: "Maya",
    lastName: "Singh",
    displayName: "Maya Singh",
    email: "maya@tron.ai",
    phone: { countryCode: "+1", number: "5550101" },
    roleCodes: ["DEVELOPER"],
    spaceCodes: ["PLATFORM", "DATA"],
    status: "Active",
    passwordResetRequired: true,
  });
  const statisticCounts: Record<string, number> = {
    "/api/v1/admin/users/statistics/active-users": 17,
    "/api/v1/admin/users/statistics/role-assignments": 29,
    "/api/v1/admin/users/statistics/space-assignments": 31,
    "/api/v1/admin/users/statistics/total-users": 42,
  };
  let created = false;

  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method === "GET" && captured.pathname in statisticCounts) {
      await fulfillJson(route, successEnvelope({ count: statisticCounts[captured.pathname] }));
      return;
    }

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users") {
      const records = created ? [directoryUser, createdUser] : [directoryUser];
      await fulfillJson(route, pagedEnvelope(records, 1, 15, records.length));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users") {
      created = true;
      await fulfillJson(route, successEnvelope(createdUser), 201);
      return;
    }

    await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
  });

  return {
    createdUser,
    directoryUser,
    statisticCounts,
  };
}

async function routeResetPasswordApi(
  page: Page,
  requests: CapturedAdminRequest[],
  resetData: unknown = {
    userCode: "USER_ALEX_JOHNSON",
    temporaryPassword: backendTemporaryPassword,
    passwordResetRequired: true,
  },
) {
  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users") {
      await fulfillJson(route, pagedEnvelope([alexUserRecord]));
      return;
    }

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON") {
      await fulfillJson(route, successEnvelope(alexUserRecord));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/password/reset") {
      await fulfillJson(route, successEnvelope(resetData));
      return;
    }

    await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
  });
}

async function routeDetailMutationApi(
  page: Page,
  requests: CapturedAdminRequest[],
  options: { profileDelayMs?: number; rolesDelayMs?: number; spacesDelayMs?: number; lifecycleDelayMs?: number } = {},
) {
  let user = makeUserRecord({
    id: "9007199254740993",
    userCode: "USER_ALEX_JOHNSON",
    username: "alex.johnson",
    firstName: "Alex",
    lastName: "Johnson",
    displayName: "Alex Johnson",
    email: "alex.johnson@example.com",
    phone: { countryCode: "+1", number: "5550100" },
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["ADMIN", "DEVELOPER"],
    spaceCodes: ["PLATFORM"],
    status: "Active",
  });
  const spaceOptionUser = makeUserRecord({
    id: "9007199254740994",
    userCode: "USER_DATA_SPACE_SOURCE",
    username: "data.space.source",
    firstName: "Data",
    lastName: "Source",
    displayName: "Data Space Source",
    email: "data.space.source@example.com",
    roleCodes: ["VIEWER"],
    spaceCodes: ["DATA"],
    status: "Active",
  });

  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users") {
      await fulfillJson(route, pagedEnvelope([user, spaceOptionUser]));
      return;
    }

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON") {
      await fulfillJson(route, successEnvelope(user));
      return;
    }

    if (captured.method === "PATCH" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON") {
      await delay(options.profileDelayMs ?? 0);
      const nextPhone = isRecord(captured.body) && "phone" in captured.body
        ? (captured.body.phone === null
          ? null
          : (isRecord(captured.body.phone) &&
              typeof captured.body.phone.countryCode === "string" &&
              typeof captured.body.phone.number === "string"
            ? {
                countryCode: captured.body.phone.countryCode,
                number: captured.body.phone.number,
              }
            : user.phone))
        : user.phone;
      user = makeUserRecord({
        ...user,
        displayName: isRecord(captured.body) && typeof captured.body.displayName === "string"
          ? captured.body.displayName
          : "Alex Backend Updated",
        email: isRecord(captured.body) && typeof captured.body.email === "string"
          ? captured.body.email
          : "alex.backend.updated@example.com",
        firstName: isRecord(captured.body) && typeof captured.body.firstName === "string"
          ? captured.body.firstName
          : user.firstName,
        lastName: isRecord(captured.body) && typeof captured.body.lastName === "string"
          ? captured.body.lastName
          : user.lastName,
        phone: nextPhone,
        locale: isRecord(captured.body) && typeof captured.body.locale === "string"
          ? captured.body.locale
          : user.locale,
        timezone: isRecord(captured.body) && typeof captured.body.timezone === "string"
          ? captured.body.timezone
          : user.timezone,
        updatedAt: "2026-05-13T03:00:00.000Z",
      });
      await fulfillJson(route, successEnvelope(user));
      return;
    }

    if (captured.method === "PUT" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/roles") {
      await delay(options.rolesDelayMs ?? 0);
      user = makeUserRecord({
        ...user,
        roleCodes: ["VIEWER"],
        updatedAt: "2026-05-13T03:05:00.000Z",
      });
      await fulfillJson(route, successEnvelope({
        userCode: user.userCode,
        roleAssignments: [{ roleCode: "VIEWER" }],
        updatedAt: user.updatedAt,
      }));
      return;
    }

    if (captured.method === "PUT" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/spaces") {
      await delay(options.spacesDelayMs ?? 0);
      user = makeUserRecord({
        ...user,
        spaceCodes: ["DATA"],
        updatedAt: "2026-05-13T03:10:00.000Z",
      });
      await fulfillJson(route, successEnvelope({
        userCode: user.userCode,
        spaceAssignments: [{ spaceCode: "DATA" }],
        updatedAt: user.updatedAt,
      }));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/archive") {
      await delay(options.lifecycleDelayMs ?? 0);
      user = makeUserRecord({
        ...user,
        status: "Archived",
        archivedAt: "2026-05-13T03:15:00.000Z",
        updatedAt: "2026-05-13T03:15:00.000Z",
      });
      await fulfillJson(route, successEnvelope({
        userCode: user.userCode,
        status: user.status,
        archivedAt: user.archivedAt,
        updatedAt: user.updatedAt,
      }));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/restore") {
      await delay(options.lifecycleDelayMs ?? 0);
      user = makeUserRecord({
        ...user,
        status: "Active",
        archivedAt: null,
        updatedAt: "2026-05-13T03:20:00.000Z",
      });
      await fulfillJson(route, successEnvelope({
        userCode: user.userCode,
        status: user.status,
        archivedAt: user.archivedAt,
        updatedAt: user.updatedAt,
      }));
      return;
    }

    await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
  });
}

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;
  frontendProcess = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        BROWSER: "none",
        VITE_TRON_ADMIN_DATA_SOURCE: "api",
        VITE_TRON_API_BASE_URL: "",
      },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(frontendBaseUrl);
});

test.beforeEach(async ({ page }) => {
  await routeAuthenticatedSession(page);
});

test.afterAll(async () => {
  const processToStop = frontendProcess;
  if (!processToStop || processToStop.exitCode !== null) return;

  const exited = new Promise((resolveExit) => {
    processToStop.once("exit", resolveExit);
  });
  processToStop.kill("SIGTERM");
  await Promise.race([exited, delay(2000)]);
  if (processToStop.exitCode === null) {
    processToStop.kill("SIGKILL");
  }
});

test("TDD-UM-08: admin user model uses stable user-management boundary fields", async () => {
  const modelSource = source("src/domain/admin/model/AdminModel.ts");

  expect(modelSource).toContain("userCode");
  expect(modelSource).toContain("username");
  expect(modelSource).toContain("roleCodes");
  expect(modelSource).toContain("spaceCodes");

  expect(modelSource).not.toContain("workspaces:");
  expect(modelSource).not.toContain("roles:");
});

test("TDD-UM-08: admin user statuses are narrowed to Active or Archived only", async () => {
  const mockDataSource = source("src/domain/admin/mock/data.ts");
  const usersArrayBlock = mockDataSource.match(
    /const users:\s*UserRecord\[\]\s*=\s*\[(?<users>[\s\S]*?)\];/,
  )?.groups?.users;
  expect(usersArrayBlock).toBeDefined();

  const statusLiterals = Array.from(
    usersArrayBlock!.matchAll(/status:\s*"([^"]+)"/g),
    (match) => match[1],
  );

  expect(statusLiterals.length).toBeGreaterThan(0);
  for (const status of statusLiterals) {
    expect(["Active", "Archived"]).toContain(status);
  }
});

test("TDD-UM-08: user detail navigation is keyed by userCode query boundary", async () => {
  const usersPageSource = source("src/domain/admin/pages/UsersPage.tsx");
  const detailsPageSource = source("src/domain/admin/pages/UserDetailsPage.tsx");

  expect(usersPageSource).toContain("/admin/users/detail?userCode=");
  expect(detailsPageSource).toContain("get(\"userCode\")");
});

test("TDD-UM-08: admin api data source calls backend list endpoint", async ({ page }) => {
  let usersEndpointCalled = false;

  await page.route("**/api/v1/admin/users**", async (route) => {
    usersEndpointCalled = true;
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        code: "SUCCESS",
        message: "ok",
        data: {
          current: 1,
          size: 20,
          total: 1,
          records: [
            {
              id: "9007199254740993",
              userCode: "USER_API_SOURCE",
              username: "api.source",
              firstName: "Api",
              lastName: "Source",
              displayName: "Api Source",
              email: "api.source@example.com",
              phone: null,
              avatarUrl: null,
              locale: "en-US",
              timezone: "UTC",
              roleCodes: ["ADMIN"],
              spaceCodes: ["PLATFORM"],
              status: "Active",
              passwordResetRequired: false,
              lastActiveAt: null,
              createdAt: "2026-05-13T00:00:00.000Z",
              updatedAt: "2026-05-13T00:00:00.000Z",
              archivedAt: null,
            },
          ],
        },
      }),
    });
  });

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Api Source")).toBeVisible();
  expect(usersEndpointCalled).toBe(true);
});

test("TDD-UM-08: missing or unknown userCode detail route exposes only not-found recovery", async ({ page }) => {
  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=UNKNOWN`);

  await expect(page.getByRole("heading", { exact: true, name: "User Detail" })).toBeVisible();
  await expect(page.getByLabel("Breadcrumb").getByRole("link", { name: "Users" })).toBeVisible();
  await expect(page.getByText("No user record matches UNKNOWN.")).toBeVisible();
  await expect(page.getByText("Alex Johnson")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Archive User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Restore User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Reset Password" })).toHaveCount(0);
  await expect(page.getByText("Role Codes")).toHaveCount(0);
  await expect(page.getByText("Space Codes")).toHaveCount(0);

  await page.goto(`${frontendBaseUrl}/admin/users/detail`);

  await expect(page.getByRole("heading", { exact: true, name: "User Detail" })).toBeVisible();
  await expect(page.getByLabel("Breadcrumb").getByRole("link", { name: "Users" })).toBeVisible();
  await expect(page.getByText("No user record was requested.")).toBeVisible();
  await expect(page.getByText("Alex Johnson")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Archive User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Restore User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Reset Password" })).toHaveCount(0);
  await expect(page.getByText("Role Codes")).toHaveCount(0);
  await expect(page.getByText("Space Codes")).toHaveCount(0);
});

test("TDD-UM-08: valid userCode detail route keeps the user detail journey", async ({ page }) => {
  await routeAdminUsers(page, [alexUserRecord]);

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_ALEX_JOHNSON`);

  await expect(page.getByRole("heading", { name: "Alex Johnson" })).toBeVisible();
  await expect(page.getByText("USER_ALEX_JOHNSON").first()).toBeVisible();
  await expect(page.getByRole("button", { name: "Archive User" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Reset Password" })).toBeVisible();
  await expect(page.getByText("Role Codes")).toBeVisible();
  await expect(page.getByText("Space Codes")).toBeVisible();
});

test("TDD-UM-09: directory query parameters drive search status and pagination responses", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeDirectoryQueryApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Directory User 01")).toBeVisible();
  const paginationStart = requests.length;
  await page.getByRole("button", { name: "Next page" }).click();
  await expectCapturedAdminRequest(
    requests,
    "GET /api/v1/admin/users?status=Active&current=2&size=15 after pagination",
    (request) => request.method === "GET" &&
      request.pathname === "/api/v1/admin/users" &&
      (request.searchParams.search ?? "") === "" &&
      request.searchParams.status === "Active" &&
      request.searchParams.current === "2" &&
      request.searchParams.size === "15",
    { after: paginationStart },
  );
  await expect(page.getByText("Page Two From API")).toBeVisible();

  const searchStart = requests.length;
  await page.getByPlaceholder("Filter users...").fill("server-search");
  await expectCapturedAdminRequest(
    requests,
    "GET /api/v1/admin/users?search=server-search&status=Active&current=1&size=15 after search",
    (request) => request.method === "GET" &&
      request.pathname === "/api/v1/admin/users" &&
      request.searchParams.search === "server-search" &&
      request.searchParams.status === "Active" &&
      request.searchParams.current === "1" &&
      request.searchParams.size === "15",
    { after: searchStart },
  );
  await expect(page.getByText("Server Search Match")).toBeVisible();

  const statusStart = requests.length;
  await page.getByLabel("Filter by status").selectOption("Archived");
  await expectCapturedAdminRequest(
    requests,
    "GET /api/v1/admin/users?search=server-search&status=Archived&current=1&size=15 after status filter",
    (request) => request.method === "GET" &&
      request.pathname === "/api/v1/admin/users" &&
      request.searchParams.search === "server-search" &&
      request.searchParams.status === "Archived" &&
      request.searchParams.current === "1" &&
      request.searchParams.size === "15",
    { after: statusStart },
  );
  await expect(page.getByText("Archived Server Result")).toBeVisible();
});

test("TDD-UM-09 TDD-UM-15: all status includeArchived request omits status and renders active plus archived rows", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeAllStatusIncludeArchivedApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("All Status Active")).toBeVisible();
  const allStatusStart = requests.length;
  await page.getByLabel("Filter by status").selectOption("All");
  const allStatusRequest = await expect.poll(
    () => requests.slice(allStatusStart).find((request) => request.method === "GET" && request.pathname === "/api/v1/admin/users"),
    {
      message: "Expected All Status to request GET /api/v1/admin/users",
      timeout: redExpectationTimeout,
    },
  ).toBeDefined().then(() => requests.slice(allStatusStart).find((request) => request.method === "GET" && request.pathname === "/api/v1/admin/users")) as CapturedAdminRequest;

  expect.soft(allStatusRequest.searchParams.search ?? "").toBe("");
  expect.soft(allStatusRequest.searchParams.current).toBe("1");
  expect.soft(allStatusRequest.searchParams.size).toBe("15");
  expect.soft(allStatusRequest.searchParams).not.toHaveProperty("status");
  expect.soft(allStatusRequest.searchParams.includeArchived).toBe("true");
  await expect(page.getByText("All Status Active")).toBeVisible();
  await expect.soft(page.getByText("All Status Archived")).toBeVisible({ timeout: redExpectationTimeout });
});

test("TDD-UM-10: add user form posts uniform create request and renders created user", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeCreateUserApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Alex Johnson")).toBeVisible();
  await page.getByRole("button", { name: "New" }).click();
  await expect(page, "New should open dedicated Create User route").toHaveURL(/\/admin\/users\/new$/);
  await expect(page.getByRole("heading", { name: /add user|create user/i })).toBeVisible();
  await expect(page.getByLabel(/user code/i), "Create User page should not expose manual userCode input").toHaveCount(0);
  await page.getByLabel(/^username$/i).fill("maya.singh");
  await page.getByLabel(/first name/i).fill("Maya");
  await page.getByLabel(/last name/i).fill("Singh");
  await expect(page.getByLabel(/display name/i)).toHaveValue("Maya Singh");
  await page.getByLabel(/^email$/i).fill("maya@tron.ai");

  const createStart = requests.length;
  await page.getByRole("button", { name: /create user|save user/i }).click();
  await expectCapturedAdminRequest(
    requests,
    "POST /api/v1/admin/users create request",
    (request) => request.method === "POST" && request.pathname === "/api/v1/admin/users",
    { after: createStart },
  );

  const createRequest = requests.find((request) => request.method === "POST" && request.pathname === "/api/v1/admin/users");
  expect(createRequest?.body).toEqual(expect.objectContaining({
    username: "maya.singh",
    firstName: "Maya",
    lastName: "Singh",
    displayName: "Maya Singh",
    email: "maya@tron.ai",
  }));
  expect((createRequest?.body as Record<string, unknown>)?.userCode).toBeUndefined();
  expect((createRequest?.body as Record<string, unknown>)?.roleCodes).toEqual([]);
  expect((createRequest?.body as Record<string, unknown>)?.spaceCodes).toEqual([]);
  await expect(userDirectoryLink(page, "Maya Singh")).toBeVisible();
  await expect(page.getByText("USER_MAYA_SINGH")).toBeVisible({ timeout: redExpectationTimeout });
});

test("TDD-UM-11: export users produces CSV download with stable headers and content", async ({ page }) => {
  await routeAdminUsers(page, [
    makeUserRecord({
      id: "734103000000006666",
      userCode: "USER_EXPORT_ALMA",
      username: "alma.export",
      firstName: "Alma",
      lastName: "Export",
      displayName: "Alma Export",
      email: "alma.export@tron.ai",
      roleCodes: ["DEVELOPER"],
      spaceCodes: ["DATA"],
      status: "Active",
    }),
  ]);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Alma Export")).toBeVisible();
  const downloadPromise = page.waitForEvent("download", { timeout: 2500 });
  await page.getByRole("button", { name: "Export", exact: true }).click();
  const download = await downloadPromise;
  const downloadPath = await download.path();

  expect(download.suggestedFilename()).toMatch(/users.*\.csv$/i);
  expect(downloadPath).toBeTruthy();

  const content = readFileSync(downloadPath!, "utf8");
  expect(content).toContain("userCode,username,displayName,email,status,roleCodes,spaceCodes");
  expect(content).toContain("USER_EXPORT_ALMA");
  expect(content).toContain("alma.export@tron.ai");
});

test("TDD-UM-12: reset password calls backend reset endpoint and renders returned temporary password", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeResetPasswordApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_ALEX_JOHNSON`);

  await expect(page.getByRole("heading", { name: "Alex Johnson" })).toBeVisible();
  await page.getByRole("button", { name: "Reset Password" }).click();
  const resetStart = requests.length;
  await page.getByRole("button", { name: "Confirm Reset" }).click();
  await expectCapturedAdminRequest(
    requests,
    "POST /api/v1/admin/users/USER_ALEX_JOHNSON/password/reset",
    (request) => request.method === "POST" &&
      request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/password/reset",
    { after: resetStart },
  );
  await expect(page.getByText(backendTemporaryPassword)).toBeVisible();
});

test("TDD-UM-12: reset response contract rejects missing temporary password without client fallback", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeResetPasswordApi(page, requests, {
    userCode: "USER_ALEX_JOHNSON",
    passwordResetRequired: true,
  });

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_ALEX_JOHNSON`);

  await expect(page.getByRole("heading", { name: "Alex Johnson" })).toBeVisible();
  await page.getByRole("button", { name: "Reset Password" }).click();
  const resetStart = requests.length;
  await page.getByRole("button", { name: "Confirm Reset" }).click();
  await expectCapturedAdminRequest(
    requests,
    "POST /api/v1/admin/users/USER_ALEX_JOHNSON/password/reset",
    (request) => request.method === "POST" &&
      request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/password/reset",
    { after: resetStart },
  );

  await expect.soft(
    page.getByText(/temporary password.*not returned|reset response/i),
    "Malformed reset success envelopes should produce a visible contract error.",
  ).toBeVisible({ timeout: redExpectationTimeout });
  await expect.soft(
    page.locator("body"),
    "Malformed reset success envelopes must not display a client-generated TRON fallback password.",
  ).not.toContainText(/TRON-/);
});

test("TDD-UM-12: reset password copy feedback is visible and close removes the temporary password", async ({ page }) => {
  await routeResetPasswordApi(page, []);

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_ALEX_JOHNSON`);

  await page.getByRole("button", { name: "Reset Password" }).click();
  await page.getByRole("button", { name: "Confirm Reset" }).click();
  const temporaryPassword = page.locator("code").first();
  await expect(temporaryPassword).toBeVisible();
  await expect(temporaryPassword).toHaveText(backendTemporaryPassword);

  await page.getByRole("button", { name: "Copy" }).click();
  const copyFeedback = page.getByText("Temporary password copied", { exact: true });
  await expect(copyFeedback).toBeVisible({ timeout: redExpectationTimeout });
  await expect(copyFeedback, "Copy feedback should be visible to operators, not sr-only").not.toHaveClass(/(?:^|\s)sr-only(?:\s|$)/, { timeout: redExpectationTimeout });

  await page.getByRole("button", { name: "Close" }).click();
  await expect(temporaryPassword).toBeHidden();
});

test("TDD-UM-13: detail mutations submit profile PATCH and render response data", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeDetailMutationApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_ALEX_JOHNSON`);

  await expect(page.getByRole("heading", { name: "Alex Johnson" })).toBeVisible();
  const editProfileButton = page.getByRole("button", { name: /edit profile/i });
  await expect(editProfileButton, "User detail should expose a profile edit action").toBeVisible({ timeout: redExpectationTimeout });
  await editProfileButton.click();
  const displayNameInput = page.getByLabel(/display name/i);
  await expect(displayNameInput, "Profile edit should expose display-name input").toBeVisible({ timeout: redExpectationTimeout });
  await displayNameInput.fill("Alex Backend Updated");
  await page.getByLabel(/^email$/i).fill("alex.backend.updated@example.com");

  const patchStart = requests.length;
  await page.getByRole("button", { name: /save profile|update profile/i }).click();
  await expectCapturedAdminRequest(
    requests,
    "PATCH /api/v1/admin/users/USER_ALEX_JOHNSON profile update",
    (request) => request.method === "PATCH" &&
      request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON",
    { after: patchStart },
  );
  await expect(page.getByRole("heading", { name: "Alex Backend Updated" })).toBeVisible();
  await expect(page.getByText("alex.backend.updated@example.com")).toBeVisible();
});

test("TDD-UM-13: detail mutations submit role space archive and restore backend endpoints", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeDetailMutationApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_ALEX_JOHNSON`);

  await expect(page.getByRole("heading", { name: "Alex Johnson" })).toBeVisible();
  const roleCodes = page.getByLabel("Role codes for Alex Johnson");
  const spaceCodes = page.getByLabel("Space codes for Alex Johnson");
  const rolesStart = requests.length;
  await roleCodes.getByRole("button", { name: "Viewer", exact: true }).click();
  await expectCapturedAdminRequest(
    requests,
    "PUT /api/v1/admin/users/USER_ALEX_JOHNSON/roles",
    (request) => request.method === "PUT" &&
      request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/roles",
    { after: rolesStart, soft: true },
  );
  await expect.soft(roleCodes.getByRole("button", { name: "Admin", exact: true })).toHaveAttribute("aria-pressed", "false");
  await expect.soft(roleCodes.getByRole("button", { name: "Viewer", exact: true })).toHaveAttribute("aria-pressed", "true");

  const spacesStart = requests.length;
  await spaceCodes.getByRole("button", { name: "DATA", exact: true }).click();
  await expectCapturedAdminRequest(
    requests,
    "PUT /api/v1/admin/users/USER_ALEX_JOHNSON/spaces",
    (request) => request.method === "PUT" &&
      request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/spaces",
    { after: spacesStart, soft: true },
  );
  await expect.soft(spaceCodes.getByRole("button", { name: "PLATFORM", exact: true })).toHaveAttribute("aria-pressed", "false");
  await expect.soft(spaceCodes.getByRole("button", { name: "DATA", exact: true })).toHaveAttribute("aria-pressed", "true");

  const archiveStart = requests.length;
  await page.getByRole("button", { name: "Archive User" }).click();
  await expectCapturedAdminRequest(
    requests,
    "POST /api/v1/admin/users/USER_ALEX_JOHNSON/archive",
    (request) => request.method === "POST" &&
      request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/archive",
    { after: archiveStart, soft: true },
  );
  await expect(page.getByText("Archived", { exact: true })).toBeVisible();

  const restoreStart = requests.length;
  await page.getByRole("button", { name: "Restore User" }).click();
  await expectCapturedAdminRequest(
    requests,
    "POST /api/v1/admin/users/USER_ALEX_JOHNSON/restore",
    (request) => request.method === "POST" &&
      request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/restore",
    { after: restoreStart, soft: true },
  );
  await expect(page.getByText("Active", { exact: true })).toBeVisible();
});

test("consistency refactor TDD-UM-16: statistic cards call four APIs and render data.count", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  const { statisticCounts } = await routeConsistencyRefactorApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Sam Directory")).toBeVisible();
  for (const pathname of Object.keys(statisticCounts)) {
    await expectCapturedAdminRequest(
      requests,
      `GET ${pathname}`,
      (request) => request.method === "GET" && request.pathname === pathname,
    );
  }

  await expect(page.getByText("Users", { exact: true }).locator("xpath=ancestor::div[contains(@class, 'rounded-2xl')][1]")).toContainText("42");
  await expect(page.getByText("Active", { exact: true }).locator("xpath=ancestor::div[contains(@class, 'rounded-2xl')][1]")).toContainText("17");
  await expect(page.getByText("Role Assignments", { exact: true }).locator("xpath=ancestor::div[contains(@class, 'rounded-2xl')][1]")).toContainText("29");
  await expect(page.getByText("Space Assignments", { exact: true }).locator("xpath=ancestor::div[contains(@class, 'rounded-2xl')][1]")).toContainText("31");
});

test("consistency refactor TDD-UM-17: directory renders User Contact columns and Space Assignments copy", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  const layoutRedTimeout = 1000;
  await routeConsistencyRefactorApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByRole("columnheader", { name: "User" })).toBeVisible();
  await expect.soft(page.getByRole("columnheader", { name: "Contact" }), "Directory must expose a Contact column for email and phone.").toBeVisible({ timeout: layoutRedTimeout });
  await expect.soft(page.getByText("Space Assignments", { exact: true }), "Assignment card label must use Space Assignments copy.").toBeVisible({ timeout: layoutRedTimeout });
  await expect.soft(page.getByText("Space Links", { exact: true }), "Old Space Links copy must be removed.").toHaveCount(0, { timeout: layoutRedTimeout });

  const samRow = page.getByRole("row").filter({ hasText: "Sam Directory" });
  await expect(samRow).toBeVisible();
  const userCell = samRow.getByRole("cell").nth(0);
  await expect(userCell).toContainText("Sam Directory");
  await expect.soft(userCell, "User column should not include username.").not.toContainText("sam.directory", { timeout: layoutRedTimeout });
  await expect.soft(userCell, "User column should not include email.").not.toContainText("sam.directory@tron.ai", { timeout: layoutRedTimeout });

  const contactCell = samRow.getByRole("cell").nth(1);
  await expect.soft(contactCell, "Contact column should include email.").toContainText("sam.directory@tron.ai", { timeout: layoutRedTimeout });
  await expect.soft(contactCell, "Contact column should include phone country code.").toContainText("+1", { timeout: layoutRedTimeout });
  await expect.soft(contactCell, "Contact column should include phone number.").toContainText("5550199", { timeout: layoutRedTimeout });
});

test("consistency refactor TDD-UM-18: New navigates to create page and submits from page owner", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeConsistencyRefactorApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Sam Directory")).toBeVisible();
  await page.getByRole("button", { name: "New" }).click();
  await expect(page, "New must navigate to the dedicated create route.").toHaveURL(/\/admin\/users\/new$/);
  await expect(page.getByRole("heading", { name: /add user|create user/i })).toBeVisible();
  await expect(page.getByLabel(/user code/i), "Create User page should not expose manual userCode input").toHaveCount(0);

  await page.getByLabel(/^username$/i).fill("maya.singh");
  await page.getByLabel(/first name/i).fill("Maya");
  await page.getByLabel(/last name/i).fill("Singh");
  await expect(page.getByLabel(/display name/i)).toHaveValue("Maya Singh");
  await page.getByLabel(/^email$/i).fill("maya@tron.ai");

  const createStart = requests.length;
  await page.getByRole("button", { name: /create user|save user/i }).click();
  await expectCapturedAdminRequest(
    requests,
    "POST /api/v1/admin/users from CreateUserPage",
    (request) => request.method === "POST" && request.pathname === "/api/v1/admin/users",
    { after: createStart },
  );

  const createRequest = requests.find((request, index) => index >= createStart && request.method === "POST" && request.pathname === "/api/v1/admin/users");
  expect(createRequest?.body).toEqual(expect.objectContaining({
    displayName: "Maya Singh",
    email: "maya@tron.ai",
    firstName: "Maya",
    lastName: "Singh",
    username: "maya.singh",
    roleCodes: [],
    spaceCodes: [],
  }));
  expect((createRequest?.body as Record<string, unknown>)?.userCode).toBeUndefined();
  await expect(page.getByText(/user created|created successfully|maya singh created/i)).toBeVisible({ timeout: redExpectationTimeout });
  await expect(page, "Successful create should return to the directory.").toHaveURL(/\/admin\/users$/);
  await expect(userDirectoryLink(page, "Maya Singh")).toBeVisible();
  await expect(page.getByText("USER_MAYA_SINGH")).toBeVisible({ timeout: redExpectationTimeout });
});

test("toast feedback TDD-UM-19: user-management pages use shared toast source contract", async () => {
  const sourceContracts = [
    {
      label: "UsersPage",
      path: "src/domain/admin/pages/UsersPage.tsx",
      forbiddenLocalState: [
        "const [notice, setNotice]",
        "setNotice(",
      ],
    },
    {
      label: "CreateUserPage",
      path: "src/domain/admin/pages/CreateUserPage.tsx",
      forbiddenLocalState: [
        "const [feedback, setFeedback]",
        "setFeedback(",
      ],
    },
    {
      label: "UserDetailsPage",
      path: "src/domain/admin/pages/UserDetailsPage.tsx",
      forbiddenLocalState: [
        "const [notice, setNotice]",
        "const [copyFeedback, setCopyFeedback]",
        "setNotice(",
        "setCopyFeedback(",
      ],
    },
  ];

  for (const contract of sourceContracts) {
    const pageSource = source(contract.path);
    expect.soft(pageSource.includes("useToast"), `${contract.label} must import or call the shared useToast hook.`).toBe(true);
    expect.soft(/\buseToast\(\)/.test(pageSource), `${contract.label} must call useToast() for operator feedback.`).toBe(true);
    expect.soft(pageSource.includes("ToastProvider"), `${contract.label} must not mount its own ToastProvider wrapper.`).toBe(false);

    for (const localState of contract.forbiddenLocalState) {
      expect.soft(
        pageSource.includes(localState),
        `${contract.label} must not keep page-local feedback state: ${localState}`,
      ).toBe(false);
    }
  }
});

test("toast feedback TDD-UM-19: Users export renders shared top-right toast", async ({ page }) => {
  await routeAdminUsers(page, [
    makeUserRecord({
      id: "734103000000006701",
      userCode: "USER_TOAST_EXPORT",
      username: "toast.export",
      firstName: "Toast",
      lastName: "Export",
      displayName: "Toast Export",
      email: "toast.export@tron.ai",
      roleCodes: ["DEVELOPER"],
      spaceCodes: ["DATA"],
      status: "Active",
    }),
  ]);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Toast Export")).toBeVisible();
  const downloadPromise = page.waitForEvent("download", { timeout: redExpectationTimeout });
  await page.getByRole("button", { name: "Export", exact: true }).click();
  await downloadPromise;
  await expectSharedTopRightToast(page, /users exported/i, "Users export feedback");
});

test("toast feedback TDD-UM-19: Create User success renders shared top-right toast", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeCreateUserApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users/new`);

  await expect(page.getByLabel(/user code/i), "Create User page should not expose manual userCode input").toHaveCount(0);
  await page.getByLabel(/^username$/i).fill("maya.singh");
  await page.getByLabel(/first name/i).fill("Maya");
  await page.getByLabel(/last name/i).fill("Singh");
  await expect(page.getByLabel(/display name/i)).toHaveValue("Maya Singh");
  await page.getByLabel(/^email$/i).fill("maya@tron.ai");

  await page.getByRole("button", { name: /create user|save user/i }).click();
  await expectCapturedAdminRequest(
    requests,
    "POST /api/v1/admin/users for shared toast create feedback",
    (request) => request.method === "POST" && request.pathname === "/api/v1/admin/users",
  );
  const createRequest = requests.find((request) => request.method === "POST" && request.pathname === "/api/v1/admin/users");
  expect((createRequest?.body as Record<string, unknown>)?.userCode).toBeUndefined();
  await expect(page, "Create success should still return to the directory before toast assertion.").toHaveURL(/\/admin\/users$/);
  await expectSharedTopRightToast(page, /Maya Singh created successfully/i, "Create User success feedback");
  await expect(page.getByText("USER_MAYA_SINGH")).toBeVisible({ timeout: redExpectationTimeout });
});

test("toast feedback TDD-UM-19: User Detail reset copy renders shared top-right toast", async ({ page }) => {
  await routeResetPasswordApi(page, []);

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_ALEX_JOHNSON`);

  await page.getByRole("button", { name: "Reset Password" }).click();
  await page.getByRole("button", { name: "Confirm Reset" }).click();
  await expect(page.locator("code").first()).toHaveText(backendTemporaryPassword);

  await page.getByRole("button", { name: "Copy" }).click();
  await expectSharedTopRightToast(page, /Temporary password copied/i, "User Detail reset copy feedback");
});

test("loading feedback TDD-UM-20: Export exposes action-local pending state and toast completion", async ({ page }) => {
  await routeAdminUsers(page, [
    makeUserRecord({
      id: "734103000000006702",
      userCode: "USER_LOADING_EXPORT",
      username: "loading.export",
      firstName: "Loading",
      lastName: "Export",
      displayName: "Loading Export",
      email: "loading.export@tron.ai",
      roleCodes: ["DEVELOPER"],
      spaceCodes: ["DATA"],
      status: "Active",
    }),
  ]);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  const exportButton = page.getByRole("button", { name: /export|exporting/i }).first();
  await expect(page.getByText("Loading Export")).toBeVisible();
  const downloadPromise = page.waitForEvent("download", { timeout: redExpectationTimeout });
  await exportButton.click();
  await expect.soft(exportButton, "Export should be disabled while the CSV action is pending.").toBeDisabled({ timeout: 300 });
  await expect.soft(exportButton, "Export should expose action-local loading copy while pending.").toContainText(/exporting|loading|working/i, { timeout: 300 });
  await downloadPromise;
  await expectSharedTopRightToast(page, /users exported/i, "Export completion feedback");
});

test("loading feedback TDD-UM-20: Create User submit disables locally while pending and resolves through toast", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeCreateUserApi(page, requests, { createDelayMs: 700 });

  await page.goto(`${frontendBaseUrl}/admin/users/new`);

  await expect(page.getByLabel(/user code/i), "Create User page should not expose manual userCode input").toHaveCount(0);
  await page.getByLabel(/^username$/i).fill("maya.singh");
  await page.getByLabel(/first name/i).fill("Maya");
  await page.getByLabel(/last name/i).fill("Singh");
  await expect(page.getByLabel(/display name/i)).toHaveValue("Maya Singh");
  await page.getByLabel(/^email$/i).fill("maya@tron.ai");

  const submitButton = page.getByRole("button", { name: /create user|creating|save user/i });
  await submitButton.click();
  await expectCapturedAdminRequest(
    requests,
    "delayed POST /api/v1/admin/users create request",
    (request) => request.method === "POST" && request.pathname === "/api/v1/admin/users",
  );
  const createRequest = requests.find((request) => request.method === "POST" && request.pathname === "/api/v1/admin/users");
  expect((createRequest?.body as Record<string, unknown>)?.userCode).toBeUndefined();
  await expect.soft(submitButton, "Create User submit should be disabled while the create request is pending.").toBeDisabled({ timeout: redExpectationTimeout });
  await expect.soft(submitButton, "Create User submit should show local loading copy while pending.").toContainText(/creating|loading|saving/i, { timeout: redExpectationTimeout });
  await expect(page, "Create success should return to the directory before toast assertion.").toHaveURL(/\/admin\/users$/);
  await expectSharedTopRightToast(page, /Maya Singh created successfully/i, "Create User completion feedback");
  await expect(page.getByText("USER_MAYA_SINGH")).toBeVisible({ timeout: redExpectationTimeout });
});

test("loading feedback TDD-UM-20: Save Profile disables locally while pending and resolves through toast", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeDetailMutationApi(page, requests, { profileDelayMs: 700 });

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_ALEX_JOHNSON`);

  await page.getByRole("button", { name: /edit profile/i }).click();
  await page.getByLabel(/display name/i).fill("Alex Backend Updated");
  await page.getByLabel(/^email$/i).fill("alex.backend.updated@example.com");

  const saveButton = page.getByRole("button", { name: /save profile|saving profile|loading/i });
  const patchStart = requests.length;
  await saveButton.click();
  await expectCapturedAdminRequest(
    requests,
    "delayed PATCH /api/v1/admin/users/USER_ALEX_JOHNSON profile update",
    (request) => request.method === "PATCH" &&
      request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON",
    { after: patchStart },
  );
  await expect.soft(saveButton, "Save Profile should be disabled while the profile request is pending.").toBeDisabled({ timeout: 300 });
  await expect.soft(saveButton, "Save Profile should show action-local loading copy while pending.").toContainText(/saving|loading|updating/i, { timeout: 300 });
  await expect(page.getByRole("heading", { name: "Alex Backend Updated" })).toBeVisible();
  await expectSharedTopRightToast(page, /profile updated/i, "Save Profile completion feedback");
});

test("operator refinement TDD-UM-21: debounced search with empty-query skipping and stale-response handling", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  const baselineRecord = makeUserRecord({
    id: "734103002743185700",
    userCode: "USER_DIRECTORY_01",
    username: "directory.01",
    firstName: "Directory",
    lastName: "User",
    displayName: "Directory User 01",
    email: "directory.01@tron.ai",
    roleCodes: ["VIEWER"],
    spaceCodes: ["PLATFORM"],
    status: "Active",
  });
  const staleRecord = makeUserRecord({
    id: "734103002743185701",
    userCode: "USER_STALE_ALPHA",
    username: "search.stale",
    firstName: "Server",
    lastName: "Stale",
    displayName: "Alpha Stale Match",
    email: "search.stale@tron.ai",
    roleCodes: ["ADMIN"],
    spaceCodes: ["PLATFORM"],
    status: "Active",
  });
  const freshRecord = makeUserRecord({
    id: "734103002743185702",
    userCode: "USER_ALPHA_MATCH",
    username: "search.alpha",
    firstName: "Server",
    lastName: "Match",
    displayName: "Alpha Fresh Match",
    email: "search.alpha@tron.ai",
    roleCodes: ["ADMIN"],
    spaceCodes: ["PLATFORM"],
    status: "Active",
  });

  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method !== "GET" || captured.pathname !== "/api/v1/admin/users") {
      await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
      return;
    }

    const search = captured.searchParams.search ?? "";
    if (search === "") {
      await delay(20);
      await fulfillJson(route, pagedEnvelope([baselineRecord], 1, 15, 1));
      return;
    }

    if (search === "alpha") {
      await delay(100);
      await fulfillJson(route, pagedEnvelope([freshRecord], 1, 15, 1));
      return;
    }

    if (search === "stale") {
      await delay(480);
      await fulfillJson(route, pagedEnvelope([staleRecord], 1, 15, 1));
      return;
    }

    await delay(20);
    await fulfillJson(route, pagedEnvelope([baselineRecord], 1, 15, 1));
  });

  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page.getByText("Directory User 01")).toBeVisible();

  const searchInput = page.getByPlaceholder("Filter users...");
  const requestBaseline = requests.length;

  await searchInput.fill("");
  await delay(250);
  expect(requests.slice(requestBaseline).some((request) => request.method === "GET" && request.pathname === "/api/v1/admin/users" && request.searchParams.search === "")).toBe(false);

  await searchInput.fill("a");
  await delay(20);
  await searchInput.fill("al");
  await delay(20);
  await searchInput.fill("alp");
  await delay(20);
  await searchInput.fill("alpha");
  await delay(20);

  await delay(700);
  const nonEmptySearchTerms = requests
    .slice(requestBaseline)
    .filter((request) => request.method === "GET" && request.pathname === "/api/v1/admin/users" && request.searchParams.search)
    .map((request) => request.searchParams.search);
  expect(nonEmptySearchTerms.length).toBeGreaterThan(0);
  expect(new Set(nonEmptySearchTerms).size).toBeLessThanOrEqual(1);

  const staleStart = requests.length;
  await searchInput.fill("stale");
  await delay(20);
  await searchInput.fill("alpha");
  await expectCapturedAdminRequest(
    requests,
    "alpha search request from directory",
    (request) => request.pathname === "/api/v1/admin/users" && request.searchParams.search === "alpha",
    { after: staleStart },
  );
  await expect(page.getByText("Alpha Fresh Match")).toBeVisible({ timeout: redExpectationTimeout });
  await delay(650);
  await expect(page.getByText("Alpha Fresh Match")).toBeVisible({ timeout: redExpectationTimeout });
  await expect(page.getByText("Alpha Stale Match")).toHaveCount(0, { timeout: redExpectationTimeout });
});

test("operator refinement TDD-UM-22: directory exposes archive and restore actions from row action menu", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  let isArchived = false;
  const directoryUser = makeUserRecord({
    id: "734103002743185800",
    userCode: "USER_DIRECTORY_OPERATOR",
    username: "operator.directory",
    firstName: "Operator",
    lastName: "Directory",
    displayName: "Operator Directory",
    email: "operator.directory@tron.ai",
    roleCodes: ["DEVELOPER"],
    spaceCodes: ["PLATFORM"],
    status: "Active",
  });

  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users") {
      const record = makeUserRecord({
        ...directoryUser,
        status: isArchived ? "Archived" : "Active",
        archivedAt: isArchived ? "2026-05-13T10:10:10.000Z" : null,
      });
      await fulfillJson(route, pagedEnvelope([record], 1, 15, 1));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users/USER_DIRECTORY_OPERATOR/archive") {
      isArchived = true;
      await fulfillJson(route, successEnvelope({ userCode: directoryUser.userCode, status: "Archived", archivedAt: "2026-05-13T10:10:10.000Z" }));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users/USER_DIRECTORY_OPERATOR/restore") {
      isArchived = false;
      await fulfillJson(route, successEnvelope({ userCode: directoryUser.userCode, status: "Active", archivedAt: null }));
      return;
    }

    await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
  });

  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page.getByText("Operator Directory")).toBeVisible();
  const userRow = page.getByRole("row").filter({ hasText: "Operator Directory" }).first();

  const actionMenuButton = userRow.getByRole("button", { name: /open actions for operator directory/i });
  await actionMenuButton.click();
  const archiveAction = page.getByRole("menuitem", { name: /archive/i });
  const restoreAction = page.getByRole("menuitem", { name: /restore/i });

  await expect(archiveAction, "Archive action should be available inside the row action menu").toBeVisible({ timeout: redExpectationTimeout });
  const archiveStart = requests.length;
  await archiveAction.click();
  await expectCapturedAdminRequest(
    requests,
    "row archive action should call archive endpoint from directory menu",
    (request) => request.method === "POST" && request.pathname === "/api/v1/admin/users/USER_DIRECTORY_OPERATOR/archive",
    { after: archiveStart },
  );
  await expect(userRow.getByText("Archived")).toBeVisible({ timeout: redExpectationTimeout });

  await actionMenuButton.click();
  await expect(restoreAction, "Restore action should be available inside the row action menu").toBeVisible({ timeout: redExpectationTimeout });
  const restoreStart = requests.length;
  await restoreAction.click();
  await expectCapturedAdminRequest(
    requests,
    "row restore action should call restore endpoint from directory menu",
    (request) => request.method === "POST" && request.pathname === "/api/v1/admin/users/USER_DIRECTORY_OPERATOR/restore",
    { after: restoreStart },
  );
  await expect(userRow.getByText("Active")).toBeVisible({ timeout: redExpectationTimeout });
});

test("operator refinement TDD-UM-23: detail inline save should keep userCode and username read-only controls while editing mutable fields", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  let detailUser = makeUserRecord({
    ...alexUserRecord,
    phone: { countryCode: "+1", number: "5550100" },
    locale: "en-US",
    timezone: "America/Los_Angeles",
    updatedAt: "2026-05-13T00:00:00.000Z",
  });

  await routeAuthenticatedSession(page);
  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users") {
      await fulfillJson(route, pagedEnvelope([detailUser], 1, 100, 1));
      return;
    }

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON") {
      await fulfillJson(route, successEnvelope(detailUser));
      return;
    }

    if (captured.method === "PATCH" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON") {
      const nextPhone = isRecord(captured.body) && "phone" in captured.body
        ? (captured.body.phone === null
          ? null
          : (isRecord(captured.body.phone) &&
              typeof captured.body.phone.countryCode === "string" &&
              typeof captured.body.phone.number === "string"
            ? {
                countryCode: captured.body.phone.countryCode,
                number: captured.body.phone.number,
              }
            : detailUser.phone))
        : detailUser.phone;

      detailUser = makeUserRecord({
        ...detailUser,
        displayName: isRecord(captured.body) && typeof captured.body.displayName === "string"
          ? captured.body.displayName
          : detailUser.displayName,
        email: isRecord(captured.body) && typeof captured.body.email === "string"
          ? captured.body.email
          : detailUser.email,
        firstName: isRecord(captured.body) && typeof captured.body.firstName === "string"
          ? captured.body.firstName
          : detailUser.firstName,
        lastName: isRecord(captured.body) && typeof captured.body.lastName === "string"
          ? captured.body.lastName
          : detailUser.lastName,
        phone: nextPhone,
        locale: isRecord(captured.body) && typeof captured.body.locale === "string"
          ? captured.body.locale
          : detailUser.locale,
        timezone: isRecord(captured.body) && typeof captured.body.timezone === "string"
          ? captured.body.timezone
          : detailUser.timezone,
        updatedAt: "2026-05-13T03:00:00.000Z",
      });
      await fulfillJson(route, successEnvelope(detailUser));
      return;
    }

    await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
  });

  await signInToProtectedRoute(page, "/admin/users/detail?userCode=USER_ALEX_JOHNSON");

  await expect(page.getByRole("heading", { name: "Alex Johnson" })).toBeVisible();
  await expect(page.getByText("Phone Country Code", { exact: true })).toBeVisible();
  await expect(page.getByText("Phone", { exact: true })).toBeVisible();
  await expect(page.getByText("Locale", { exact: true })).toBeVisible();
  await expect(page.getByText("Timezone", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: /edit profile/i }).click();
  const userCodeInput = page.getByRole("textbox", { name: /user code/i });
  const usernameInput = page.getByRole("textbox", { name: /^username$/i });
  await expect(userCodeInput, "userCode should remain visible in edit mode as a read-only control").toBeVisible();
  await expect(usernameInput, "username should remain visible in edit mode as a read-only control").toBeVisible();
  await expect.soft(userCodeInput, "userCode should not be editable in inline detail editing").not.toBeEditable({ timeout: redExpectationTimeout });
  await expect.soft(usernameInput, "username should not be editable in inline detail editing").not.toBeEditable({ timeout: redExpectationTimeout });
  await expect.soft(userCodeInput, "userCode should be visually emphasized as non-editable.").toHaveClass(/bg-\[var\(--color-surface-container\)\]/);
  await expect.soft(usernameInput, "username should be visually emphasized as non-editable.").toHaveClass(/bg-\[var\(--color-surface-container\)\]/);

  const firstNameInput = page.getByLabel(/first name/i);
  const lastNameInput = page.getByLabel(/last name/i);
  const phoneCountryCodeInput = page.getByRole("textbox", { name: /phone country code/i });
  const phoneInput = page.getByRole("textbox", { name: /^phone$/i });
  const localeInput = page.getByRole("textbox", { name: /locale/i });
  const timezoneInput = page.getByRole("textbox", { name: /timezone/i });
  await expect(firstNameInput, "first name should be an inline editable field in detail profile mode").toBeVisible();
  await expect(firstNameInput, "first name should remain editable in detail profile mode").toBeEditable();
  await expect(lastNameInput, "last name should be an inline editable field in detail profile mode").toBeVisible();
  await expect(lastNameInput, "last name should remain editable in detail profile mode").toBeEditable();
  await expect(page.getByLabel(/display name/i), "display name should remain editable in detail profile mode").toBeEditable();
  await expect(page.getByLabel(/^email$/i), "email should remain editable in detail profile mode").toBeEditable();
  await expect(phoneCountryCodeInput, "phone country code should be editable in detail profile mode").toBeEditable();
  await expect(phoneInput, "phone should be editable in detail profile mode").toBeEditable();
  await expect(localeInput, "locale should be editable in detail profile mode").toBeEditable();
  await expect(timezoneInput, "timezone should be editable in detail profile mode").toBeEditable();
  await firstNameInput.fill("Alex Inline");
  await lastNameInput.fill("Johnson Inline");
  await page.getByLabel(/display name/i).fill("Alex Inline Updated");
  await page.getByLabel(/^email$/i).fill("alex.inline.updated@example.com");
  await phoneCountryCodeInput.fill("+86");
  await phoneInput.fill("13800138000");
  await localeInput.fill("zh-CN");
  await timezoneInput.fill("Asia/Shanghai");

  const patchStart = requests.length;
  await page.getByRole("button", { name: /save profile|saving profile/i }).click();
  await expectCapturedAdminRequest(
    requests,
    "PATCH /api/v1/admin/users/USER_ALEX_JOHNSON should only include mutable fields",
    (request) => request.method === "PATCH" && request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON",
    { after: patchStart },
  );

  const patchRequest = requests.find((request, index) => index >= patchStart && request.method === "PATCH" && request.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON");
  expect(patchRequest).toBeDefined();
  expect(isRecord(patchRequest!.body)).toBe(true);
  expect((patchRequest!.body as Record<string, unknown>)).not.toHaveProperty("userCode");
  expect((patchRequest!.body as Record<string, unknown>)).not.toHaveProperty("username");
  expect((patchRequest!.body as Record<string, unknown>)).toEqual(expect.objectContaining({
    firstName: "Alex Inline",
    lastName: "Johnson Inline",
    displayName: "Alex Inline Updated",
    email: "alex.inline.updated@example.com",
    phone: {
      countryCode: "+86",
      number: "13800138000",
    },
    locale: "zh-CN",
    timezone: "Asia/Shanghai",
  }));
  await expect(page.getByRole("heading", { name: "Alex Inline Updated" })).toBeVisible();
  await expect(page.getByText("alex.inline.updated@example.com")).toBeVisible();
  await expect(page.getByText("+86", { exact: true })).toBeVisible();
  await expect(page.getByText("13800138000", { exact: true })).toBeVisible();
  await expect(page.getByText("zh-CN", { exact: true })).toBeVisible();
  await expect(page.getByText("Asia/Shanghai", { exact: true })).toBeVisible();
  await expect(page.getByText(formatUiTimestamp("2026-05-13T03:00:00.000Z"), { exact: true })).toBeVisible();
});

test("github oauth2 users expose source type without overloading last name", async ({ page }) => {
  const githubUser = makeUserRecord({
    userCode: "USR-CLOUDSEN",
    sourceType: "GitHub",
    username: "cloudsen",
    firstName: "CloudS3n",
    lastName: "CloudS3n",
    displayName: "CloudS3n",
    email: "cloudsen@example.com",
    roleCodes: ["VIEWER"],
    spaceCodes: ["PLATFORM"],
  });

  await routeAuthenticatedSession(page);
  await routeAdminUsers(page, [githubUser]);
  await signInToProtectedRoute(page, "/admin/users/detail?userCode=USR-CLOUDSEN");

  const detailFieldValue = (label: string) =>
    page.getByText(label, { exact: true }).locator("xpath=following-sibling::*[1]");

  await expect(detailFieldValue("Source Type")).toHaveText("GitHub");
  await expect(detailFieldValue("Last Name")).toHaveText("CloudS3n");
  await expect(detailFieldValue("Last Name")).not.toHaveText("GitHub");
});

test("operator refinement TDD-UM-24: create flow should omit userCode input/payload and render backend-generated userCode", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  const generatedUser = makeUserRecord({
    id: "734103002743185901",
    userCode: "USER_GENERATED_ALEX",
    username: "generated.alex",
    firstName: "Generated",
    lastName: "Alex",
    displayName: "Generated Alex",
    email: "generated.alex@tron.ai",
    phone: { countryCode: "+86", number: "13800138000" },
    locale: "zh-CN",
    timezone: "Asia/Shanghai",
    roleCodes: [],
    spaceCodes: [],
    status: "Active",
  });
  let createdBody: unknown = {};

  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users") {
      await fulfillJson(route, pagedEnvelope([alexUserRecord, generatedUser], 1, 15, 2));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users") {
      createdBody = captured.body;
      await fulfillJson(route, successEnvelope(generatedUser), 201);
      return;
    }

    await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
  });

  await page.goto(`${frontendBaseUrl}/admin/users/new`);

  await expect(page.getByRole("heading", { name: "Create User" })).toBeVisible();
  await expect(page.getByLabel(/user code/i)).toHaveCount(0);
  await expect(page.getByLabel(/role code/i)).toHaveCount(0);
  await expect(page.getByLabel(/space codes/i)).toHaveCount(0);
  await page.getByLabel(/^username$/i).fill("generated.alex");
  await page.getByLabel(/first name/i).fill("Generated");
  await expect(page.getByLabel(/display name/i)).toHaveValue("Generated");
  await page.getByLabel(/last name/i).fill("Alex");
  await expect(page.getByLabel(/display name/i)).toHaveValue("Generated Alex");
  await page.getByLabel(/^email$/i).fill("generated.alex@tron.ai");
  await page.getByLabel(/phone country code/i).fill("+86");
  await page.getByLabel(/^phone$/i).fill("13800138000");
  await page.getByLabel(/locale/i).fill("zh-CN");
  await page.getByLabel(/timezone/i).fill("Asia/Shanghai");

  const createStart = requests.length;
  await page.getByRole("button", { name: /create user|save user/i }).click();
  await expectCapturedAdminRequest(
    requests,
    "POST create request should not include userCode in payload",
    (request) => request.method === "POST" && request.pathname === "/api/v1/admin/users",
    { after: createStart },
  );

  expect(isRecord(createdBody)).toBe(true);
  expect((createdBody as Record<string, unknown>)).not.toHaveProperty("userCode");
  expect((createdBody as Record<string, unknown>)).toEqual(expect.objectContaining({
    username: "generated.alex",
    firstName: "Generated",
    lastName: "Alex",
    displayName: "Generated Alex",
    email: "generated.alex@tron.ai",
    phone: {
      countryCode: "+86",
      number: "13800138000",
    },
    locale: "zh-CN",
    timezone: "Asia/Shanghai",
    roleCodes: [],
    spaceCodes: [],
  }));
  await expect(page, "Create success should return to the directory and display generated code.").toHaveURL(/\/admin\/users$/);
  await expect(page.getByText("USER_GENERATED_ALEX")).toBeVisible({ timeout: redExpectationTimeout });
});

test("operator refinement TDD-UM-25: archived user detail loads through the direct detail endpoint and keeps restore state visible", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  let archivedUser = makeUserRecord({
    id: "734103112812822528",
    userCode: "USER_RINA_PATEL",
    username: "rina.patel",
    firstName: "Rina",
    lastName: "Patel",
    displayName: "Rina Patel",
    email: "rina@tron.ai",
    phone: { countryCode: "+1", number: "5550102" },
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["WORKSPACE_ADMIN", "VIEWER"],
    spaceCodes: ["SUPPORT", "PLATFORM"],
    status: "Archived",
    archivedAt: "2026-05-13T03:15:00.000Z",
    updatedAt: "2026-05-13T03:15:00.000Z",
  });

  await page.route("**/api/v1/admin/users**", async (route) => {
    const captured = captureAdminRequest(route);
    requests.push(captured);

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users") {
      await fulfillJson(route, pagedEnvelope([alexUserRecord, archivedUser], 1, 100, 2));
      return;
    }

    if (captured.method === "GET" && captured.pathname === "/api/v1/admin/users/USER_RINA_PATEL") {
      await fulfillJson(route, successEnvelope(archivedUser));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users/USER_RINA_PATEL/restore") {
      archivedUser = makeUserRecord({
        ...archivedUser,
        status: "Active",
        archivedAt: null,
        updatedAt: "2026-05-13T03:20:00.000Z",
      });
      await fulfillJson(route, successEnvelope(archivedUser));
      return;
    }

    await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
  });

  await page.goto(`${frontendBaseUrl}/admin/users/detail?userCode=USER_RINA_PATEL`);

  await expect(page.getByRole("heading", { name: "Rina Patel" })).toBeVisible();
  await expect(page.getByText("User not found")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Restore User" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Archive User" })).toHaveCount(0);
  await expect(page.getByText("Archived", { exact: true })).toBeVisible();
  await expect(page.getByText(formatUiTimestamp("2026-05-13T03:15:00.000Z"), { exact: true })).toBeVisible();

  const restoreStart = requests.length;
  await page.getByRole("button", { name: "Restore User" }).click();
  await expectCapturedAdminRequest(
    requests,
    "archived detail restore should call restore endpoint",
    (request) => request.method === "POST" && request.pathname === "/api/v1/admin/users/USER_RINA_PATEL/restore",
    { after: restoreStart },
  );
  await expect(page.getByText("Active", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Archive User" })).toBeVisible();
  await expect(page.getByText(formatUiTimestamp("2026-05-13T03:20:00.000Z"), { exact: true })).toBeVisible();
});
