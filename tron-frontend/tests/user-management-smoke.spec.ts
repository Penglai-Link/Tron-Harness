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

function pagedEnvelope(records: unknown[], current = 1, size = 20, total = records.length) {
  return successEnvelope({
    current,
    size,
    total,
    records,
  });
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

async function routeCreateUserApi(page: Page, requests: CapturedAdminRequest[]) {
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

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/password/reset") {
      await fulfillJson(route, successEnvelope(resetData));
      return;
    }

    await fulfillJson(route, { code: "TEST_ROUTE_NOT_FOUND", message: `${captured.method} ${captured.pathname}`, data: {} }, 404);
  });
}

async function routeDetailMutationApi(page: Page, requests: CapturedAdminRequest[]) {
  let user = makeUserRecord({
    id: "9007199254740993",
    userCode: "USER_ALEX_JOHNSON",
    username: "alex.johnson",
    firstName: "Alex",
    lastName: "Johnson",
    displayName: "Alex Johnson",
    email: "alex.johnson@example.com",
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
      user = makeUserRecord({
        ...user,
        displayName: "Alex Backend Updated",
        email: "alex.backend.updated@example.com",
        updatedAt: "2026-05-13T03:00:00.000Z",
      });
      await fulfillJson(route, successEnvelope(user));
      return;
    }

    if (captured.method === "PUT" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/roles") {
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
      }));
      return;
    }

    if (captured.method === "POST" && captured.pathname === "/api/v1/admin/users/USER_ALEX_JOHNSON/restore") {
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

  await expect(page.getByRole("heading", { name: "User Detail" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Back to Users" })).toBeVisible();
  await expect(page.getByText("No user record matches UNKNOWN.")).toBeVisible();
  await expect(page.getByText("Alex Johnson")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Archive User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Restore User" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Reset Password" })).toHaveCount(0);
  await expect(page.getByText("Role Codes")).toHaveCount(0);
  await expect(page.getByText("Space Codes")).toHaveCount(0);

  await page.goto(`${frontendBaseUrl}/admin/users/detail`);

  await expect(page.getByRole("heading", { name: "User Detail" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Back to Users" })).toBeVisible();
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
    "GET /api/v1/admin/users?search=&status=Active&current=2&size=15 after pagination",
    (request) => request.method === "GET" &&
      request.pathname === "/api/v1/admin/users" &&
      request.searchParams.search === "" &&
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

test("TDD-UM-10: add user form posts uniform create request and renders created user", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeCreateUserApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Alex Johnson")).toBeVisible();
  await page.getByRole("button", { name: "Add User" }).click();
  const userCodeInput = page.getByLabel(/user code/i);
  await expect(userCodeInput, "Add User should open a create-user form").toBeVisible({ timeout: redExpectationTimeout });
  await userCodeInput.fill("USER_MAYA_SINGH");
  await page.getByLabel(/^username$/i).fill("maya.singh");
  await page.getByLabel(/first name/i).fill("Maya");
  await page.getByLabel(/last name/i).fill("Singh");
  await page.getByLabel(/display name/i).fill("Maya Singh");
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
    userCode: "USER_MAYA_SINGH",
    username: "maya.singh",
    firstName: "Maya",
    lastName: "Singh",
    displayName: "Maya Singh",
    email: "maya@tron.ai",
  }));
  expect(isRecord(createRequest?.body) && Array.isArray(createRequest.body.roleCodes)).toBe(true);
  expect(isRecord(createRequest?.body) && Array.isArray(createRequest.body.spaceCodes)).toBe(true);
  await expect(page.getByText("Maya Singh")).toBeVisible();
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

test("consistency refactor TDD-UM-18: Add User navigates to create page and submits from page owner", async ({ page }) => {
  const requests: CapturedAdminRequest[] = [];
  await routeConsistencyRefactorApi(page, requests);

  await page.goto(`${frontendBaseUrl}/admin/users`);

  await expect(page.getByText("Sam Directory")).toBeVisible();
  await page.getByRole("button", { name: "Add User" }).click();
  await expect(page, "Add User must navigate to the dedicated create route.").toHaveURL(/\/admin\/users\/new$/);
  await expect(page.getByRole("heading", { name: /add user|create user/i })).toBeVisible();

  await page.getByLabel(/user code/i).fill("USER_MAYA_SINGH");
  await page.getByLabel(/^username$/i).fill("maya.singh");
  await page.getByLabel(/first name/i).fill("Maya");
  await page.getByLabel(/last name/i).fill("Singh");
  await page.getByLabel(/display name/i).fill("Maya Singh");
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
    userCode: "USER_MAYA_SINGH",
    username: "maya.singh",
  }));
  await expect(page.getByText(/user created|created successfully|maya singh created/i)).toBeVisible({ timeout: redExpectationTimeout });
  await expect(page, "Successful create should return to the directory.").toHaveURL(/\/admin\/users$/);
  await expect(page.getByText("Maya Singh")).toBeVisible();
});
