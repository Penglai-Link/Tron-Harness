import {
  expect,
  test,
  type Page,
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

test.describe.configure({ timeout: 45_000 });

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

async function blockAuthApi(page: Page) {
  const authRequests: string[] = [];

  await page.route("**/api/v1/auth/**", async (route) => {
    authRequests.push(route.request().url());
    await route.abort();
  });

  return authRequests;
}

async function submitLogin(page: Page) {
  await page.getByLabel(/passkey|password/i).press("Enter");
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
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_AUTH_DATA_SOURCE: "mock",
        VITE_TRON_DATA_SOURCE: "mock",
      },
      stdio: "pipe",
    },
  );

  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(`${frontendBaseUrl}/login`);
});

test.afterAll(async () => {
  if (!frontendProcess) return;
  frontendProcess.kill("SIGTERM");
  await new Promise<void>((resolveProcess) => {
    frontendProcess?.once("exit", () => resolveProcess());
    setTimeout(() => resolveProcess(), 5_000);
  });
});

test("opens protected routes, logs out, and signs back in without auth API requests in mock mode", async ({ page }) => {
  const authRequests = await blockAuthApi(page);

  await page.goto(`${frontendBaseUrl}/admin/users`);
  await expect(page).toHaveURL(/\/admin\/users$/);
  await expect(page.getByRole("heading", { name: /users/i }).first()).toBeVisible();
  await expect(page.getByRole("navigation", { name: /tron primary navigation/i })).toBeVisible();

  await page.getByRole("button", { name: /open user menu/i }).click();
  await page.getByRole("button", { name: /^log out$/i }).click();
  await expect(page).toHaveURL(/\/login/);
  await expect(page.getByRole("button", { name: /oauth2/i })).toBeVisible();

  const loginInput = page.getByLabel(/operator id|username|email/i);
  const passwordInput = page.getByLabel(/passkey|password/i);
  await expect(loginInput).toHaveValue("auth.operator");
  await expect(passwordInput).toHaveValue("TronAuthPass123!");
  await expect(loginInput).toHaveAttribute("placeholder", "Enter username or email");

  await loginInput.fill("auth.operator");
  await passwordInput.fill("TronAuthPass123!");
  await submitLogin(page);

  await expect(page).toHaveURL(/\/admin\/users$/);
  expect(authRequests).toEqual([]);
});
