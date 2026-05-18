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
import { createServer } from "node:net";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");
let frontendProcess: ChildProcessWithoutNullStreams | undefined;
let frontendLogs = "";
let frontendBaseUrl = "";

const authenticatedSession = {
  authMethod: "internal",
  authenticated: true,
  provider: null,
  user: {
    displayName: "Auth Operator",
    email: "auth.operator@tron.ai",
    roleCodes: ["ADMIN", "WORKSPACE_ADMIN"],
    spaceCodes: ["ENTERPRISE", "PLATFORM"],
    status: "Active",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
  },
};

const permissionDomains = ["Workspace", "Models", "Tasks", "Memory", "Skills", "MCP"];
const roleCatalog = [
  {
    id: "734300000000000001",
    code: "ADMIN",
    name: "Admin",
    description: "Full System Access",
    kind: "System",
    managedBy: "Platform",
    permissions: Object.fromEntries(permissionDomains.map((domain) => [domain, ["R", "W", "D", "A"]])),
    updatedAt: "2026-05-16T00:00:00Z",
  },
  {
    id: "734300000000000002",
    code: "WORKSPACE_ADMIN",
    name: "Workspace Admin",
    description: "Workspace administration",
    kind: "System",
    managedBy: "Workspace",
    permissions: {
      Workspace: ["R", "W"],
      Models: ["R", "W"],
      Tasks: ["R", "W"],
      Memory: ["R"],
      Skills: ["R", "W"],
      MCP: ["R"],
    },
    updatedAt: "2026-05-16T00:00:00Z",
  },
  {
    id: "734300000000000003",
    code: "DEVELOPER",
    name: "Developer",
    description: "Build and operate applications",
    kind: "System",
    managedBy: "Platform",
    permissions: {
      Workspace: ["R"],
      Models: ["R", "W"],
      Tasks: ["R", "W"],
      Memory: ["R"],
      Skills: ["R", "W"],
      MCP: ["R"],
    },
    updatedAt: "2026-05-16T00:00:00Z",
  },
  {
    id: "734300000000000004",
    code: "VIEWER",
    name: "Viewer",
    description: "Read-only Access",
    kind: "System",
    managedBy: "Platform",
    permissions: Object.fromEntries(permissionDomains.map((domain) => [domain, ["R"]])),
    updatedAt: "2026-05-16T00:00:00Z",
  },
] as const;

const users = [
  {
    id: "734102998871252992",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
    firstName: "Auth",
    lastName: "Operator",
    displayName: "Auth Operator",
    email: "auth.operator@tron.ai",
    phone: { countryCode: "+1", number: "5550100" },
    avatarUrl: null,
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["ADMIN", "WORKSPACE_ADMIN"],
    spaceCodes: ["ENTERPRISE", "PLATFORM"],
    status: "Active",
    passwordResetRequired: false,
    lastActiveAt: "2026-05-16T08:30:00Z",
    createdAt: "2026-05-16T08:00:00Z",
    updatedAt: "2026-05-16T08:00:00Z",
    archivedAt: null,
  },
  {
    id: "734103002743185408",
    userCode: "USER_MAYA_SINGH",
    username: "maya.singh",
    firstName: "Maya",
    lastName: "Singh",
    displayName: "Maya Singh",
    email: "maya@tron.ai",
    phone: { countryCode: "+1", number: "5550101" },
    avatarUrl: null,
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["DEVELOPER"],
    spaceCodes: ["PLATFORM", "DATA"],
    status: "Active",
    passwordResetRequired: false,
    lastActiveAt: "2026-05-16T08:14:00Z",
    createdAt: "2026-05-16T08:40:00Z",
    updatedAt: "2026-05-16T08:40:00Z",
    archivedAt: null,
  },
  {
    id: "9007199254740993",
    userCode: "USER_CHEN_WEI",
    username: "chen.wei",
    firstName: "Chen",
    lastName: "Wei",
    displayName: "Chen Wei",
    email: "chen@tron.ai",
    phone: null,
    avatarUrl: null,
    locale: "en-US",
    timezone: "Asia/Shanghai",
    roleCodes: ["VIEWER"],
    spaceCodes: ["ENTERPRISE", "DATA"],
    status: "Active",
    passwordResetRequired: false,
    lastActiveAt: null,
    createdAt: "2026-05-16T03:20:00Z",
    updatedAt: "2026-05-16T03:20:00Z",
    archivedAt: null,
  },
  {
    id: "734103112812822528",
    userCode: "USER_RINA_PATEL",
    username: "rina.patel",
    firstName: "Rina",
    lastName: "Patel",
    displayName: "Rina Patel",
    email: "rina@tron.ai",
    phone: { countryCode: "+1", number: "5550102" },
    avatarUrl: null,
    locale: "en-US",
    timezone: "America/Los_Angeles",
    roleCodes: ["WORKSPACE_ADMIN", "VIEWER"],
    spaceCodes: ["SUPPORT", "PLATFORM"],
    status: "Archived",
    passwordResetRequired: false,
    lastActiveAt: "2026-04-28T17:15:00Z",
    createdAt: "2026-04-12T10:00:00Z",
    updatedAt: "2026-05-08T19:00:00Z",
    archivedAt: "2026-05-08T19:00:00Z",
  },
] as const;

type WorkspaceNode = {
  archivedAt: string | null;
  children: WorkspaceNode[];
  createdAt: string;
  depth: number;
  description: string | null;
  id: string;
  name: string;
  parentWorkspaceCode: string | null;
  status: "Active" | "Archived";
  updatedAt: string;
  workspaceCode: string;
};

type WorkspaceUserGrant = {
  grantedAt: string;
  isOwner: boolean;
  userCode: string;
};

type WorkspaceRoleGrant = {
  grantedAt: string;
  roleCode: string;
};

type CapturedRequest = {
  method: string;
  pathname: string;
  searchParams: Record<string, string>;
};

let nextWorkspaceId = 734400000000000100n;
let nextGeneratedWorkspaceCode = 1;

function nowTimestamp() {
  return "2026-05-16T12:00:00Z";
}

function generatedWorkspaceCode(nodes: WorkspaceNode[]) {
  for (let attempt = 0; attempt < 16; attempt += 1) {
    const workspaceCode = `WS-${(nextGeneratedWorkspaceCode++).toString(16).padStart(16, "0").toUpperCase()}`;
    if (!findNode(nodes, workspaceCode)) return workspaceCode;
  }

  throw new Error("Workspace code could not be generated.");
}

function delay(ms: number) {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms);
  });
}

function appendFrontendLog(chunk: Buffer) {
  frontendLogs = `${frontendLogs}${chunk.toString()}`.slice(-5000);
}

function successEnvelope(data: unknown) {
  return {
    code: "SUCCESS",
    message: "ok",
    data,
  };
}

function pagedEnvelope(records: unknown[], current = 1, size = records.length || 100, total = records.length) {
  return successEnvelope({
    current,
    size,
    total,
    records,
  });
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
}

function sharedToastMessage(page: Page, message: string | RegExp) {
  return page.locator('[aria-label="Notifications"]').getByRole("status").filter({ hasText: message }).first();
}

function captureRequest(route: Route): CapturedRequest {
  const request = route.request();
  const url = new URL(request.url());

  return {
    method: request.method(),
    pathname: url.pathname,
    searchParams: Object.fromEntries(url.searchParams.entries()),
  };
}

function cloneNode(node: WorkspaceNode): WorkspaceNode {
  return {
    ...node,
    children: node.children.map(cloneNode),
  };
}

function flattenTree(nodes: WorkspaceNode[]): WorkspaceNode[] {
  return nodes.flatMap((node) => [node, ...flattenTree(node.children)]);
}

function findNode(nodes: WorkspaceNode[], workspaceCode: string): WorkspaceNode | null {
  return flattenTree(nodes).find((node) => node.workspaceCode === workspaceCode) ?? null;
}

function normalizeTree(nodes: WorkspaceNode[], parentWorkspaceCode: string | null = null, depth = 1): WorkspaceNode[] {
  return nodes.map((node) => ({
    ...node,
    parentWorkspaceCode,
    depth,
    children: normalizeTree(node.children, node.workspaceCode, depth + 1),
  }));
}

function workspaceDescendants(node: WorkspaceNode): Set<string> {
  const codes = new Set<string>([node.workspaceCode]);
  node.children.forEach((child) => {
    workspaceDescendants(child).forEach((value) => codes.add(value));
  });
  return codes;
}

function subtreeSpan(node: WorkspaceNode): number {
  return node.children.length ? 1 + Math.max(...node.children.map(subtreeSpan)) : 0;
}

function matchesWorkspaceSearch(node: WorkspaceNode, search: string) {
  if (!search.trim()) return true;
  const searchable = [
    node.workspaceCode,
    node.name,
    node.description ?? "",
    node.status,
  ].join(" ").toLowerCase();
  return searchable.includes(search.trim().toLowerCase());
}

function roleLabel(roleCode: string) {
  return roleCatalog.find((role) => role.code === roleCode)?.name ?? roleCode;
}

function activeUsers() {
  return users.filter((user) => user.status === "Active");
}

function effectiveUsersFor(workspaceCode: string, directUserGrants: Record<string, WorkspaceUserGrant[]>, roleGrants: Record<string, WorkspaceRoleGrant[]>) {
  const direct = directUserGrants[workspaceCode] ?? [];
  const grantedRoles = roleGrants[workspaceCode] ?? [];
  const grantedRoleCodes = new Set(grantedRoles.map((grant) => grant.roleCode));
  const directMap = new Map(direct.map((grant) => [grant.userCode, grant]));

  return activeUsers()
    .map((user) => {
      const directEntry = directMap.get(user.userCode);
      const matchedRoleCodes = user.roleCodes.filter((roleCode) => grantedRoleCodes.has(roleCode));
      if (!directEntry && !matchedRoleCodes.length) return null;
      return {
        userCode: user.userCode,
        displayName: user.displayName,
        email: user.email,
        source: directEntry && matchedRoleCodes.length ? "DirectAndRole" : directEntry ? "Direct" : "Role",
        matchedRoleCodes,
        isOwner: directEntry?.isOwner ?? false,
      };
    })
    .filter((record): record is NonNullable<typeof record> => record !== null);
}

function treeRecord(node: WorkspaceNode, directUserGrants: Record<string, WorkspaceUserGrant[]>, roleGrants: Record<string, WorkspaceRoleGrant[]>) {
  const children = node.children.map((child) => treeRecord(child, directUserGrants, roleGrants));
  return {
    id: node.id,
    workspaceCode: node.workspaceCode,
    name: node.name,
    description: node.description,
    parentWorkspaceCode: node.parentWorkspaceCode,
    depth: node.depth,
    status: node.status,
    childCount: children.length,
    directUserCount: (directUserGrants[node.workspaceCode] ?? []).length,
    roleGrantCount: (roleGrants[node.workspaceCode] ?? []).length,
    effectiveUserCount: effectiveUsersFor(node.workspaceCode, directUserGrants, roleGrants).length,
    children,
  };
}

function filterTree(records: ReturnType<typeof treeRecord>[], search: string, status: string | undefined, includeArchived: boolean) {
  const visibleStatus = includeArchived && !status ? null : status ?? "Active";
  return records
    .map((record) => {
      const children = filterTree(record.children, search, status, includeArchived);
      const statusMatches = visibleStatus === null || record.status === visibleStatus;
      const searchMatches = matchesWorkspaceSearch(record, search);
      if (!(statusMatches && searchMatches) && !children.length) return null;
      return {
        ...record,
        children,
      };
    })
    .filter((record): record is ReturnType<typeof treeRecord> => record !== null);
}

function buildLineage(nodes: WorkspaceNode[], workspaceCode: string) {
  const flat = flattenTree(nodes);
  const lineage: Array<{ workspaceCode: string; name: string; depth: number }> = [];
  let current = flat.find((node) => node.workspaceCode === workspaceCode) ?? null;
  while (current) {
    lineage.push({
      workspaceCode: current.workspaceCode,
      name: current.name,
      depth: current.depth,
    });
    current = current.parentWorkspaceCode ? flat.find((node) => node.workspaceCode === current?.parentWorkspaceCode) ?? null : null;
  }
  return lineage.reverse();
}

function detailRecord(nodes: WorkspaceNode[], workspaceCode: string, directUserGrants: Record<string, WorkspaceUserGrant[]>, roleGrants: Record<string, WorkspaceRoleGrant[]>) {
  const node = findNode(nodes, workspaceCode);
  if (!node) return null;
  const children = node.children.map((child) => treeRecord(child, directUserGrants, roleGrants));

  return {
    id: node.id,
    workspaceCode: node.workspaceCode,
    name: node.name,
    description: node.description,
    parentWorkspaceCode: node.parentWorkspaceCode,
    depth: node.depth,
    status: node.status,
    lineage: buildLineage(nodes, workspaceCode),
    children,
    directUserCount: (directUserGrants[node.workspaceCode] ?? []).length,
    roleGrantCount: (roleGrants[node.workspaceCode] ?? []).length,
    effectiveUserCount: effectiveUsersFor(node.workspaceCode, directUserGrants, roleGrants).length,
    createdAt: node.createdAt,
    updatedAt: node.updatedAt,
    archivedAt: node.archivedAt,
  };
}

function listUserAccess(workspaceCode: string, directUserGrants: Record<string, WorkspaceUserGrant[]>) {
  const records = (directUserGrants[workspaceCode] ?? []).map((grant) => {
    const user = users.find((record) => record.userCode === grant.userCode)!;
    return {
      userCode: grant.userCode,
      displayName: user.displayName,
      email: user.email,
      isOwner: grant.isOwner,
      grantedAt: grant.grantedAt,
    };
  });
  return {
    current: 1,
    size: 100,
    total: records.length,
    records,
  };
}

function listRoleAccess(workspaceCode: string, roleGrants: Record<string, WorkspaceRoleGrant[]>) {
  return (roleGrants[workspaceCode] ?? []).map((grant) => ({
    roleCode: grant.roleCode,
    roleName: roleLabel(grant.roleCode),
    matchedUserCount: activeUsers().filter((user) => user.roleCodes.includes(grant.roleCode)).length,
    grantedAt: grant.grantedAt,
  }));
}

function statisticsFor(nodes: WorkspaceNode[], directUserGrants: Record<string, WorkspaceUserGrant[]>, roleGrants: Record<string, WorkspaceRoleGrant[]>) {
  const flat = flattenTree(nodes);
  return {
    totalWorkspaces: flat.length,
    activeWorkspaces: flat.filter((node) => node.status === "Active").length,
    directUserGrants: Object.values(directUserGrants).reduce((total, grants) => total + grants.length, 0),
    roleGrants: Object.values(roleGrants).reduce((total, grants) => total + grants.length, 0),
  };
}

function removeNode(nodes: WorkspaceNode[], workspaceCode: string): { nextNodes: WorkspaceNode[]; removedNode: WorkspaceNode | null } {
  let removedNode: WorkspaceNode | null = null;
  const nextNodes = nodes
    .map((node) => {
      if (node.workspaceCode === workspaceCode) {
        removedNode = node;
        return null;
      }

      if (!node.children.length) return node;

      const nested = removeNode(node.children, workspaceCode);
      if (nested.removedNode) {
        removedNode = nested.removedNode;
        return {
          ...node,
          children: nested.nextNodes,
        };
      }

      return node;
    })
    .filter((node): node is WorkspaceNode => node !== null);

  return { nextNodes, removedNode };
}

function insertNode(nodes: WorkspaceNode[], parentWorkspaceCode: string | null, node: WorkspaceNode): WorkspaceNode[] {
  if (!parentWorkspaceCode) return [...nodes, node];

  return nodes.map((current) => {
    if (current.workspaceCode === parentWorkspaceCode) {
      return {
        ...current,
        children: [...current.children, node],
      };
    }

    return current.children.length
      ? {
        ...current,
        children: insertNode(current.children, parentWorkspaceCode, node),
      }
      : current;
  });
}

function updateNode(nodes: WorkspaceNode[], workspaceCode: string, updater: (node: WorkspaceNode) => WorkspaceNode): WorkspaceNode[] {
  return nodes.map((node) => {
    if (node.workspaceCode === workspaceCode) return updater(node);
    if (!node.children.length) return node;
    return {
      ...node,
      children: updateNode(node.children, workspaceCode, updater),
    };
  });
}

function updateSubtreeStatus(node: WorkspaceNode, status: "Active" | "Archived", archivedAt: string | null): WorkspaceNode {
  return {
    ...node,
    status,
    archivedAt,
    updatedAt: nowTimestamp(),
    children: node.children.map((child) => updateSubtreeStatus(child, status, archivedAt)),
  };
}

async function routeAuthenticatedSession(page: Page) {
  const accessToken = "test-access-token-1";

  await page.route("**/api/v1/auth/providers", async (route) => {
    await fulfillJson(route, successEnvelope({
      providers: [
        { code: "internal", name: "Internal Account", type: "internal" },
        { code: "github", name: "GitHub", type: "oauth2" },
      ],
    }));
  });

  await page.route("**/api/v1/auth/refresh", async (route) => {
    await fulfillJson(route, successEnvelope({
      accessToken,
      tokenType: "Bearer",
      expiresIn: 900,
      session: authenticatedSession,
    }));
  });

  await page.route("**/api/v1/auth/session", async (route) => {
    await fulfillJson(route, successEnvelope(authenticatedSession));
  });

  await page.route("**/api/v1/auth/logout", async (route) => {
    await fulfillJson(route, successEnvelope({ loggedOut: true }));
  });
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

test.beforeAll(async () => {
  const port = await reservePort();
  frontendBaseUrl = `http://127.0.0.1:${port}`;

  frontendProcess = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)],
    {
      cwd: frontendRoot,
      env: {
        ...process.env,
        CI: "1",
        VITE_TRON_ADMIN_DATA_SOURCE: "api",
        VITE_TRON_DATA_SOURCE: "api",
      },
      stdio: "pipe",
    },
  );

  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/admin/workspace`);
});

test.afterAll(async () => {
  if (!frontendProcess || frontendProcess.killed) return;

  frontendProcess.kill("SIGTERM");
  await new Promise((resolveProcess) => frontendProcess?.once("exit", resolveProcess));
});

test("workspace management page supports hierarchy and separated access grants", async ({ page }) => {
  const requests: CapturedRequest[] = [];
  let workspaceNodes: WorkspaceNode[] = normalizeTree([
    {
      id: "734400000000000001",
      workspaceCode: "ENTERPRISE",
      name: "Enterprise",
      description: "Global governance workspace",
      parentWorkspaceCode: null,
      depth: 1,
      status: "Active",
      createdAt: "2026-05-16T00:00:00Z",
      updatedAt: "2026-05-16T00:00:00Z",
      archivedAt: null,
      children: [
        {
          id: "734400000000000002",
          workspaceCode: "PLATFORM",
          name: "Platform",
          description: "Core platform runtime workspace",
          parentWorkspaceCode: "ENTERPRISE",
          depth: 2,
          status: "Active",
          createdAt: "2026-05-16T00:00:00Z",
          updatedAt: "2026-05-16T00:00:00Z",
          archivedAt: null,
          children: [
            {
              id: "734400000000000003",
              workspaceCode: "TRON_CORE",
              name: "Tron Core",
              description: "Main Tron console workspace",
              parentWorkspaceCode: "PLATFORM",
              depth: 3,
              status: "Active",
              createdAt: "2026-05-16T00:00:00Z",
              updatedAt: "2026-05-16T00:00:00Z",
              archivedAt: null,
              children: [],
            },
          ],
        },
      ],
    },
    {
      id: "734400000000000004",
      workspaceCode: "COMPLIANCE",
      name: "Compliance",
      description: "Compliance evidence workspace",
      parentWorkspaceCode: null,
      depth: 1,
      status: "Active",
      createdAt: "2026-05-16T00:00:00Z",
      updatedAt: "2026-05-16T00:00:00Z",
      archivedAt: null,
      children: [],
    },
    {
      id: "734400000000000005",
      workspaceCode: "ARCHIVE_VAULT",
      name: "Archive Vault",
      description: "Archived workspace kept for historical access",
      parentWorkspaceCode: null,
      depth: 1,
      status: "Archived",
      createdAt: "2026-05-16T00:00:00Z",
      updatedAt: "2026-05-16T00:00:00Z",
      archivedAt: "2026-05-16T00:00:00Z",
      children: [],
    },
  ]);
  let directUserGrants: Record<string, WorkspaceUserGrant[]> = {
    TRON_CORE: [{ userCode: "USER_MAYA_SINGH", isOwner: true, grantedAt: "2026-05-16T00:00:00Z" }],
  };
  let roleGrants: Record<string, WorkspaceRoleGrant[]> = {
    PLATFORM: [{ roleCode: "WORKSPACE_ADMIN", grantedAt: "2026-05-16T00:00:00Z" }],
  };

  function listWorkspaces(search: string, status: string | undefined, includeArchived: boolean) {
    workspaceNodes = normalizeTree(workspaceNodes);
    const records = workspaceNodes.map((node) => treeRecord(node, directUserGrants, roleGrants));
    return filterTree(records, search, status, includeArchived);
  }

  function getWorkspaceOrThrow(workspaceCode: string) {
    const detail = detailRecord(workspaceNodes, workspaceCode, directUserGrants, roleGrants);
    if (!detail) throw new Error(`Workspace ${workspaceCode} was not found.`);
    return detail;
  }

  function workspaceOptionRecords() {
    return flattenTree(workspaceNodes)
      .filter((node) => node.status === "Active")
      .map((node) => ({
        workspaceCode: node.workspaceCode,
        label: getWorkspaceOrThrow(node.workspaceCode).lineage.map((item) => item.name).join(" / "),
        name: node.name,
        description: node.description,
        parentWorkspaceCode: node.parentWorkspaceCode,
        depth: node.depth,
        status: node.status,
      }));
  }

  function workspaceSwitcherPayload() {
    const activeNodes = flattenTree(workspaceNodes).filter((node) => node.status === "Active");
    return {
      primarySpaces: activeNodes
        .filter((node) => node.parentWorkspaceCode === null)
        .map((node) => ({
          workspaceCode: node.workspaceCode,
          name: node.name,
          description: node.description,
          childCount: node.children.length,
          status: node.status,
        })),
      secondarySpaces: activeNodes.map((node) => {
        const detail = getWorkspaceOrThrow(node.workspaceCode);
        return {
          workspaceCode: node.workspaceCode,
          primaryWorkspaceCode: detail.lineage[0]?.workspaceCode ?? node.workspaceCode,
          label: detail.lineage.map((item) => item.name).join(" / "),
          name: node.name,
          description: node.description,
          depth: node.depth,
          childCount: node.children.length,
          effectiveUserCount: effectiveUsersFor(node.workspaceCode, directUserGrants, roleGrants).length,
          status: node.status,
        };
      }),
    };
  }

  await routeAuthenticatedSession(page);

  await page.route("**/api/v1/admin/**", async (route) => {
    const captured = captureRequest(route);
    requests.push(captured);

    try {
      if (captured.pathname === "/api/v1/admin/roles") {
        await fulfillJson(route, successEnvelope(roleCatalog));
        return;
      }

      if (captured.pathname === "/api/v1/admin/users") {
        await fulfillJson(route, pagedEnvelope([...users], 1, 200, users.length));
        return;
      }

      if (captured.pathname === "/api/v1/admin/workspaces/options") {
        await fulfillJson(route, successEnvelope(workspaceOptionRecords()));
        return;
      }

      if (captured.pathname === "/api/v1/admin/workspaces/switcher") {
        await fulfillJson(route, successEnvelope(workspaceSwitcherPayload()));
        return;
      }

      if (captured.pathname === "/api/v1/admin/workspaces") {
        if (captured.method === "POST") {
          const payload = route.request().postDataJSON() as {
            description?: string | null;
            name: string;
            parentWorkspaceCode?: string | null;
          };
          if ("workspaceCode" in payload) {
            throw new Error("workspaceCode should be generated by the backend.");
          }
          const workspaceCode = generatedWorkspaceCode(workspaceNodes);
          const parentWorkspaceCode = payload.parentWorkspaceCode ?? null;
          const parent = parentWorkspaceCode ? findNode(workspaceNodes, parentWorkspaceCode) : null;
          const depth = (parent?.depth ?? 0) + 1;
          if (depth > 3) throw new Error("Workspace hierarchy cannot exceed 3 levels.");
          const createdAt = nowTimestamp();
          workspaceNodes = insertNode(workspaceNodes, parentWorkspaceCode, {
            id: String(nextWorkspaceId++),
            workspaceCode,
            name: payload.name.trim(),
            description: payload.description?.trim() || null,
            parentWorkspaceCode,
            depth,
            status: "Active",
            createdAt,
            updatedAt: createdAt,
            archivedAt: null,
            children: [],
          });
          workspaceNodes = normalizeTree(workspaceNodes);
          await fulfillJson(route, successEnvelope(getWorkspaceOrThrow(workspaceCode)), 201);
          return;
        }

        await fulfillJson(route, successEnvelope(listWorkspaces(
          captured.searchParams.search ?? "",
          captured.searchParams.status,
          captured.searchParams.includeArchived === "true",
        )));
        return;
      }

      if (captured.pathname === "/api/v1/admin/workspaces/statistics/total-workspaces") {
        await fulfillJson(route, successEnvelope({ count: statisticsFor(workspaceNodes, directUserGrants, roleGrants).totalWorkspaces }));
        return;
      }

      if (captured.pathname === "/api/v1/admin/workspaces/statistics/active-workspaces") {
        await fulfillJson(route, successEnvelope({ count: statisticsFor(workspaceNodes, directUserGrants, roleGrants).activeWorkspaces }));
        return;
      }

      if (captured.pathname === "/api/v1/admin/workspaces/statistics/direct-user-grants") {
        await fulfillJson(route, successEnvelope({ count: statisticsFor(workspaceNodes, directUserGrants, roleGrants).directUserGrants }));
        return;
      }

      if (captured.pathname === "/api/v1/admin/workspaces/statistics/role-grants") {
        await fulfillJson(route, successEnvelope({ count: statisticsFor(workspaceNodes, directUserGrants, roleGrants).roleGrants }));
        return;
      }

      const workspaceMatch = captured.pathname.match(/^\/api\/v1\/admin\/workspaces\/([^/]+)(?:\/(.*))?$/);
      if (!workspaceMatch) {
        await fulfillJson(route, successEnvelope({}));
        return;
      }

      const workspaceCode = decodeURIComponent(workspaceMatch[1]);
      const tail = workspaceMatch[2] ?? "";

      if (!tail && captured.method === "GET") {
        await fulfillJson(route, successEnvelope(getWorkspaceOrThrow(workspaceCode)));
        return;
      }

      if (!tail && captured.method === "PATCH") {
        const payload = route.request().postDataJSON() as { description?: string | null; name: string };
        workspaceNodes = updateNode(workspaceNodes, workspaceCode, (node) => ({
          ...node,
          name: payload.name.trim(),
          description: payload.description?.trim() || null,
          updatedAt: nowTimestamp(),
        }));
        workspaceNodes = normalizeTree(workspaceNodes);
        await fulfillJson(route, successEnvelope(getWorkspaceOrThrow(workspaceCode)));
        return;
      }

      if (tail === "move" && captured.method === "POST") {
        const payload = route.request().postDataJSON() as { parentWorkspaceCode?: string | null };
        const node = findNode(workspaceNodes, workspaceCode);
        if (!node) throw new Error(`Workspace ${workspaceCode} was not found.`);
        const nextParentWorkspaceCode = payload.parentWorkspaceCode ?? null;
        if (nextParentWorkspaceCode === workspaceCode) throw new Error("Workspace cannot become its own parent.");
        const descendants = workspaceDescendants(node);
        if (nextParentWorkspaceCode && descendants.has(nextParentWorkspaceCode)) {
          throw new Error("Workspace cannot move under its own descendant.");
        }
        const parent = nextParentWorkspaceCode ? findNode(workspaceNodes, nextParentWorkspaceCode) : null;
        const nextDepth = (parent?.depth ?? 0) + 1;
        if (nextDepth + subtreeSpan(node) > 3) throw new Error("Workspace hierarchy cannot exceed 3 levels.");

        const removal = removeNode(workspaceNodes, workspaceCode);
        if (!removal.removedNode) throw new Error(`Workspace ${workspaceCode} was not found.`);
        workspaceNodes = insertNode(removal.nextNodes, nextParentWorkspaceCode, {
          ...removal.removedNode,
          parentWorkspaceCode: nextParentWorkspaceCode,
          updatedAt: nowTimestamp(),
        });
        workspaceNodes = normalizeTree(workspaceNodes);
        await fulfillJson(route, successEnvelope(getWorkspaceOrThrow(workspaceCode)));
        return;
      }

      if (tail === "archive" && captured.method === "POST") {
        const node = findNode(workspaceNodes, workspaceCode);
        if (!node) throw new Error(`Workspace ${workspaceCode} was not found.`);
        const archivedAt = nowTimestamp();
        workspaceNodes = updateNode(workspaceNodes, workspaceCode, (current) => updateSubtreeStatus(current, "Archived", archivedAt));
        workspaceNodes = normalizeTree(workspaceNodes);
        await fulfillJson(route, successEnvelope({
          workspaceCode,
          status: "Archived",
          archivedAt,
          affectedWorkspaceCount: flattenTree([findNode(workspaceNodes, workspaceCode)!]).length,
        }));
        return;
      }

      if (tail === "restore" && captured.method === "POST") {
        const node = findNode(workspaceNodes, workspaceCode);
        if (!node) throw new Error(`Workspace ${workspaceCode} was not found.`);
        if (node.parentWorkspaceCode) {
          const parent = findNode(workspaceNodes, node.parentWorkspaceCode);
          if (parent?.status === "Archived") throw new Error("Restore the parent workspace before restoring this node.");
        }
        workspaceNodes = updateNode(workspaceNodes, workspaceCode, (current) => updateSubtreeStatus(current, "Active", null));
        workspaceNodes = normalizeTree(workspaceNodes);
        await fulfillJson(route, successEnvelope({
          workspaceCode,
          status: "Active",
          archivedAt: null,
          affectedWorkspaceCount: flattenTree([findNode(workspaceNodes, workspaceCode)!]).length,
        }));
        return;
      }

      if (tail === "access/users" && captured.method === "GET") {
        await fulfillJson(route, pagedEnvelope(listUserAccess(workspaceCode, directUserGrants).records, 1, 100, listUserAccess(workspaceCode, directUserGrants).total));
        return;
      }

      if (tail === "access/users" && captured.method === "PUT") {
        const payload = route.request().postDataJSON() as { users: Array<{ isOwner: boolean; userCode: string }> };
        directUserGrants = {
          ...directUserGrants,
          [workspaceCode]: payload.users.map((grant) => ({
            userCode: grant.userCode,
            isOwner: grant.isOwner,
            grantedAt: nowTimestamp(),
          })),
        };
        await fulfillJson(route, successEnvelope(listUserAccess(workspaceCode, directUserGrants)));
        return;
      }

      if (tail === "access/roles" && captured.method === "GET") {
        await fulfillJson(route, successEnvelope(listRoleAccess(workspaceCode, roleGrants)));
        return;
      }

      if (tail === "access/roles" && captured.method === "PUT") {
        const payload = route.request().postDataJSON() as { roles: Array<{ roleCode: string }> };
        roleGrants = {
          ...roleGrants,
          [workspaceCode]: payload.roles.map((grant) => ({
            roleCode: grant.roleCode,
            grantedAt: nowTimestamp(),
          })),
        };
        await fulfillJson(route, successEnvelope(listRoleAccess(workspaceCode, roleGrants)));
        return;
      }

      if (tail === "access/effective-users" && captured.method === "GET") {
        const effectiveUsers = effectiveUsersFor(workspaceCode, directUserGrants, roleGrants);
        await fulfillJson(route, pagedEnvelope(effectiveUsers, 1, 100, effectiveUsers.length));
        return;
      }

      await fulfillJson(route, successEnvelope({}));
    } catch (error) {
      const message = error instanceof Error ? error.message : "Workspace API request failed";
      await route.fulfill({
        status: 400,
        contentType: "application/json",
        body: JSON.stringify({ code: "BAD_REQUEST", message, data: null }),
      });
    }
  });

  await page.goto(`${frontendBaseUrl}/admin/workspace`);

  await expect(page.getByRole("heading", { name: "Workspace Management" })).toBeVisible();
  await expect(page.getByText("Platform", { exact: true }).first()).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/workspaces/switcher")).toBe(true);

  const primarySpaceToggle = page.locator("header").getByRole("button", { name: /Enterprise/ }).first();
  await primarySpaceToggle.click();
  const primarySpacePanel = page.locator("div").filter({ has: page.getByText("Primary spaces", { exact: true }) }).last();
  await expect(primarySpacePanel).toBeVisible();
  await primarySpacePanel.getByRole("button", { name: /Compliance/ }).click();
  await expect(page.locator("header").getByRole("button", { name: /Compliance/ }).first()).toBeVisible();

  const workspaceUrl = page.url();
  const searchBox = page.getByRole("searchbox", { name: "Search workspaces" });
  await searchBox.fill("Compliance");
  await expect.poll(() => requests.some((request) => (
    request.pathname === "/api/v1/admin/workspaces"
    && request.method === "GET"
    && request.searchParams.search === "Compliance"
    && request.searchParams.status === "Active"
  ))).toBe(true);
  await expect(page.getByRole("button", { name: "Select workspace COMPLIANCE", exact: true })).toBeVisible();

  const requestCountBeforeClear = requests.length;
  await page.getByRole("button", { name: "Clear workspace search" }).click();
  await expect(searchBox).toHaveValue("");
  await expect.poll(() => requests.slice(requestCountBeforeClear).some((request) => (
    request.pathname === "/api/v1/admin/workspaces"
    && request.method === "GET"
    && request.searchParams.search === undefined
    && request.searchParams.status === "Active"
  ))).toBe(true);
  await expect(page.getByRole("button", { name: "Select workspace PLATFORM", exact: true })).toBeVisible();

  const statusFilterSelect = page.getByRole("combobox", { name: "Filter workspaces by status" });
  const requestCountBeforeAllStatus = requests.length;
  await statusFilterSelect.selectOption("All");
  await expect.poll(() => requests.slice(requestCountBeforeAllStatus).some((request) => (
    request.pathname === "/api/v1/admin/workspaces"
    && request.method === "GET"
    && request.searchParams.includeArchived === "true"
    && request.searchParams.status === undefined
  ))).toBe(true);
  await expect(page.getByRole("button", { name: "Select workspace ARCHIVE_VAULT", exact: true })).toBeVisible();

  const requestCountBeforeActiveStatus = requests.length;
  await statusFilterSelect.selectOption("Active");
  await expect.poll(() => requests.slice(requestCountBeforeActiveStatus).some((request) => (
    request.pathname === "/api/v1/admin/workspaces"
    && request.method === "GET"
    && request.searchParams.status === "Active"
    && request.searchParams.includeArchived === undefined
  ))).toBe(true);
  await expect(page.getByRole("button", { name: "Select workspace PLATFORM", exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Select workspace PLATFORM", exact: true }).click();
  const overviewSection = page.getByRole("combobox", { name: "Move parent" }).locator("xpath=ancestor::section[1]");
  await expect(page).toHaveURL(workspaceUrl);
  await expect(overviewSection.getByRole("textbox", { name: "Name", exact: true })).toHaveValue("Platform");

  await page.getByRole("button", { name: "Create Root", exact: true }).click();
  await page.getByRole("textbox", { name: "Workspace name" }).fill("Risk");
  await page.getByRole("textbox", { name: "Workspace description" }).fill("Risk governance workspace");
  await page.getByRole("textbox", { name: "Workspace name" }).locator("xpath=ancestor::section[1]").getByRole("button", { name: "Create", exact: true }).click();
  await expect(sharedToastMessage(page, "Workspace WS-0000000000000001 created")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/workspaces" && request.method === "POST")).toBe(true);
  await expect(overviewSection.getByRole("textbox", { name: "Name", exact: true })).toHaveValue("Risk");

  await page.getByRole("button", { name: "Create", exact: true }).click();
  await page.getByRole("textbox", { name: "Workspace name" }).fill("Risk Review");
  await page.getByRole("textbox", { name: "Workspace description" }).fill("Child workspace for risk reviews");
  await page.getByRole("textbox", { name: "Workspace name" }).locator("xpath=ancestor::section[1]").getByRole("button", { name: "Create", exact: true }).click();
  await expect(sharedToastMessage(page, "Workspace WS-0000000000000002 created")).toBeVisible();
  await expect(overviewSection.getByRole("textbox", { name: "Name", exact: true })).toHaveValue("Risk Review");

  await page.getByRole("button", { name: "Select workspace WS-0000000000000001", exact: true }).click();
  await overviewSection.getByRole("textbox", { name: "Name", exact: true }).fill("Risk Control");
  await overviewSection.getByRole("button", { name: "Save", exact: true }).click();
  await expect(sharedToastMessage(page, "Workspace WS-0000000000000001 updated")).toBeVisible();
  await expect(overviewSection.getByRole("textbox", { name: "Name", exact: true })).toHaveValue("Risk Control");

  await overviewSection.getByRole("combobox", { name: "Move parent" }).selectOption("COMPLIANCE");
  await overviewSection.getByRole("button", { name: "Move", exact: true }).click();
  await expect(sharedToastMessage(page, "Workspace WS-0000000000000001 moved")).toBeVisible();
  await expect(page.getByText("Compliance / Risk Control")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/workspaces/WS-0000000000000001/move" && request.method === "POST")).toBe(true);

  await page.getByRole("button", { name: "Select workspace PLATFORM", exact: true }).click();
  await page.getByRole("tab", { name: "Access", exact: true }).click();
  const directUserSection = page.getByRole("combobox", { name: "Add direct user access" }).locator("xpath=ancestor::section[1]");
  await directUserSection.getByRole("combobox", { name: "Add direct user access" }).selectOption("USR-AUTH-0001");
  await directUserSection.getByRole("button", { name: "Add", exact: true }).click();
  await directUserSection.locator("tr", { hasText: "Auth Operator" }).getByRole("button", { name: "Member" }).click();
  await directUserSection.getByRole("button", { name: "Save", exact: true }).click();
  await expect(sharedToastMessage(page, "Direct user access updated")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/workspaces/PLATFORM/access/users" && request.method === "PUT")).toBe(true);

  const roleAccessSection = page.getByRole("combobox", { name: "Add workspace role access" }).locator("xpath=ancestor::section[1]");
  await roleAccessSection.getByRole("combobox", { name: "Add workspace role access" }).selectOption("VIEWER");
  await roleAccessSection.getByRole("button", { name: "Add", exact: true }).click();
  await roleAccessSection.getByRole("button", { name: "Save", exact: true }).click();
  await expect(sharedToastMessage(page, "Role access updated")).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/workspaces/PLATFORM/access/roles" && request.method === "PUT")).toBe(true);

  const effectiveUsersTable = page.locator("table").last();
  await expect(effectiveUsersTable.getByText("Auth Operator", { exact: true }).first()).toBeVisible();
  await expect(effectiveUsersTable.getByText("DirectAndRole", { exact: true }).first()).toBeVisible();
  await expect(effectiveUsersTable.getByText("Chen Wei", { exact: true }).first()).toBeVisible();
  await expect(effectiveUsersTable.getByText("Viewer", { exact: true }).first()).toBeVisible();

  await statusFilterSelect.selectOption("All");
  await expect(page.getByRole("button", { name: "Select workspace WS-0000000000000001", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Select workspace WS-0000000000000001", exact: true }).click();
  await page.getByRole("button", { name: "Archive", exact: true }).click();
  await expect(sharedToastMessage(page, "WS-0000000000000001 archived")).toBeVisible();
  await expect(page.getByRole("button", { name: "Restore", exact: true })).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/workspaces/WS-0000000000000001/archive" && request.method === "POST")).toBe(true);

  await page.getByRole("button", { name: "Restore", exact: true }).click();
  await expect(sharedToastMessage(page, "WS-0000000000000001 restored")).toBeVisible();
  await expect(page.getByRole("button", { name: "Archive", exact: true })).toBeVisible();
  await expect.poll(() => requests.some((request) => request.pathname === "/api/v1/admin/workspaces/WS-0000000000000001/restore" && request.method === "POST")).toBe(true);
});
