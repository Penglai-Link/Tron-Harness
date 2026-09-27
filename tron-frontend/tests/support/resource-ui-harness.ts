import type { Page, Route } from "@playwright/test";
import { createServer } from "node:net";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  spawnManagedProcess,
  stopManagedProcess,
  type ManagedProcess,
} from "./managed-process";

const frontendRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

const authenticatedSession = {
  authMethod: "internal",
  authenticated: true,
  provider: null,
  user: {
    displayName: "Resource Operator",
    email: "resource.operator@tron.ai",
    roleCodes: ["ADMIN", "WORKSPACE_ADMIN"],
    spaceCodes: ["TRON_CORE"],
    status: "Active",
    userCode: "USR-RESOURCE-0001",
    username: "resource.operator",
  },
};

export const resourceWorkspaceSwitcher = {
  primarySpaces: [
    {
      childCount: 1,
      description: "Global governance workspace",
      name: "Enterprise",
      status: "Active",
      workspaceCode: "ENTERPRISE",
    },
  ],
  secondarySpaces: [
    {
      childCount: 1,
      depth: 1,
      description: "Global governance workspace",
      effectiveUserCount: 1,
      label: "Enterprise",
      name: "Enterprise",
      primaryWorkspaceCode: "ENTERPRISE",
      status: "Active",
      workspaceCode: "ENTERPRISE",
    },
    {
      childCount: 1,
      depth: 2,
      description: "Platform workspace",
      effectiveUserCount: 1,
      label: "Enterprise / Platform",
      name: "Platform",
      primaryWorkspaceCode: "ENTERPRISE",
      status: "Active",
      workspaceCode: "PLATFORM",
    },
    {
      childCount: 0,
      depth: 3,
      description: "Tron product workspace",
      effectiveUserCount: 1,
      label: "Enterprise / Platform / Tron Core",
      name: "Tron Core",
      primaryWorkspaceCode: "ENTERPRISE",
      status: "Active",
      workspaceCode: "TRON_CORE",
    },
  ],
};

function successEnvelope(data: unknown) {
  return { code: "SUCCESS", data, message: "ok" };
}

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    body: JSON.stringify(data),
    contentType: "application/json",
    status,
  });
}

/** Current desktop profile bootstrap, independent of a suite's product/API fixtures. */
export async function routeActiveProfileApi(page: Page) {
  await page.route(/\/api\/v1\/(?:bootstrap\/status|profiles(?:\/active\/token|\/events)?)(?:\?.*)?$/, async (route) => {
    const pathname = new URL(route.request().url()).pathname;
    // Profile-based authentication replaced the legacy auth/session bootstrap.
    const activeProfile = {
      profileId: "PROFILE-RESOURCE", personAiUserId: authenticatedSession.user.userCode,
      personAiUsername: authenticatedSession.user.displayName, status: "ACTIVE",
      createdAt: "2026-09-27T00:00:00Z", lastActivatedAt: null, avatarUrl: null,
    };
    if (pathname === "/api/v1/bootstrap/status") {
      return fulfillJson(route, successEnvelope({
        hasProfiles: true, activeProfileId: activeProfile.profileId, activeProfile,
        avatarUrl: null, account: null,
      }));
    }
    if (pathname === "/api/v1/profiles") return fulfillJson(route, successEnvelope([activeProfile]));
    if (pathname === "/api/v1/profiles/active/token") return fulfillJson(route, successEnvelope({ token: "resource-ui-token" }));
    if (pathname === "/api/v1/profiles/events") return route.fulfill({ status: 204 });

  });
}

export async function routeResourceUiShellApi(page: Page) {
  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const pathname = new URL(request.url()).pathname;


    if (pathname === "/api/v1/auth/session" && request.method() === "GET") {
      return fulfillJson(route, successEnvelope(authenticatedSession));
    }
    if (pathname === "/api/v1/auth/refresh" && request.method() === "POST") {
      return fulfillJson(route, successEnvelope({
        accessToken: "resource-ui-token",
        expiresIn: 3600,
        session: authenticatedSession,
        tokenType: "Bearer",
      }));
    }
    if (pathname === "/api/v1/admin/workspaces/switcher" && request.method() === "GET") {
      return fulfillJson(route, successEnvelope(resourceWorkspaceSwitcher));
    }

    return fulfillJson(route, {
      code: "NOT_FOUND",
      data: {},
      message: `${request.method()} ${pathname} is not mocked`,
    }, 404);
  });
  await routeActiveProfileApi(page);
}

export async function seedResourceUiShell(page: Page, locale: "en-US" | "zh-CN" = "en-US") {
  await page.addInitScript(({ selectedLocale, switcher }) => {
    window.localStorage.setItem("tron.locale", selectedLocale);
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Data: true },
        selectedPrimarySpaceId: "ENTERPRISE",
        selectedSecondarySpaceId: "TRON_CORE",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: switcher,
      },
      version: 9,
    }));
  }, { selectedLocale: locale, switcher: resourceWorkspaceSwitcher });
}

async function reservePort() {
  return new Promise<number>((resolvePort, rejectPort) => {
    const server = createServer();
    server.on("error", rejectPort);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        rejectPort(new Error("Failed to reserve a frontend test port."));
        return;
      }
      server.close(() => resolvePort(address.port));
    });
  });
}

export interface ResourceUiFrontend {
  baseUrl: string;
  logs: () => string;
  process: ManagedProcess;
}

export async function startResourceUiFrontend(
  initialPath: string,
  environment: NodeJS.ProcessEnv = {},
): Promise<ResourceUiFrontend> {
  const port = await reservePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  let logs = "";
  const managedProcess = spawnManagedProcess(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    {
      cwd: frontendRoot,
      env: { ...process.env, ...environment, CI: "1" },
      stdio: "pipe",
    },
  );
  managedProcess.stdout.on("data", (chunk: Buffer) => {
    logs = `${logs}${chunk.toString()}`.slice(-5_000);
  });
  managedProcess.stderr.on("data", (chunk: Buffer) => {
    logs = `${logs}${chunk.toString()}`.slice(-5_000);
  });

  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (managedProcess.exitCode !== null) {
      throw new Error(`Frontend dev server exited early.\n${logs}`);
    }
    try {
      if ((await fetch(`${baseUrl}${initialPath}`)).ok) {
        return { baseUrl, logs: () => logs, process: managedProcess };
      }
    } catch {
      // Vite is still starting.
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
  }

  await stopManagedProcess(managedProcess);
  throw new Error(`Frontend dev server did not become ready.\n${logs}`);
}

export async function stopResourceUiFrontend(frontend: ResourceUiFrontend | undefined) {
  if (frontend) await stopManagedProcess(frontend.process);
}
