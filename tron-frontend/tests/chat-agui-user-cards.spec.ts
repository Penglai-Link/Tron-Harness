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

import { getWorkspaceSwitcherMock } from "../src/shared/constants/workspaceSpaces";

const thisFilePath = fileURLToPath(import.meta.url);
const frontendRoot = resolve(thisFilePath, "..", "..");

let frontendProcess: ChildProcessWithoutNullStreams | undefined;
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
    spaceCodes: ["PLATFORM", "TRON_CORE"],
    status: "Active",
    userCode: "USR-AUTH-0001",
    username: "auth.operator",
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

async function stopFrontend() {
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

async function fulfillJson(route: Route, data: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
}

async function routeAuthApi(page: Page) {
  let currentSession: typeof authUser | null = null;
  let hasRefreshSession = false;
  let currentAccessToken = "";
  let nextAccessTokenId = 1;

  function issueGrant(session: typeof authUser) {
    currentSession = session;
    hasRefreshSession = true;
    currentAccessToken = `test-access-token-${nextAccessTokenId++}`;

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
      await fulfillJson(route, issueGrant(authUser));
      return;
    }

    await fulfillJson(route, unauthorizedEnvelope("AUTH_INVALID_CREDENTIALS", "invalid credentials"), 401);
  });

  await page.route("**/api/v1/auth/refresh", async (route) => {
    if (!currentSession || !hasRefreshSession) {
      await fulfillJson(route, unauthorizedEnvelope("AUTH_REFRESH_INVALID", "invalid refresh session"), 401);
      return;
    }

    await fulfillJson(route, issueGrant(currentSession));
  });
}

function escapedPathPattern(path: string) {
  return new RegExp(`${path.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`);
}

async function signIn(page: Page, targetPath: string, expectedPath = targetPath) {
  const expectedUrlPattern = escapedPathPattern(expectedPath);
  const loginUrlPattern = new RegExp(`/login\\?redirect=${encodeURIComponent(targetPath).replace(/\//g, "%2F")}$`);

  await page.goto(`${frontendBaseUrl}${targetPath}`);

  if (!expectedUrlPattern.test(page.url()) && !loginUrlPattern.test(page.url())) {
    await page.waitForURL((url) => expectedUrlPattern.test(url.href) || loginUrlPattern.test(url.href));
  }

  if (expectedUrlPattern.test(page.url())) return;

  await expect(page).toHaveURL(loginUrlPattern);

  await page.getByLabel(/operator id|username|email/i).fill("auth.operator");
  await page.getByLabel(/passkey|password/i).fill("TronAuthPass123!");
  await page.getByRole("button", { name: /sign in/i }).click();
  await expect(page).toHaveURL(expectedUrlPattern);
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
        VITE_TRON_API_BASE_URL: "",
        VITE_TRON_DATA_SOURCE: "mock",
        VITE_TRON_ADMIN_DATA_SOURCE: "mock",
        VITE_TRON_CHAT_DATA_SOURCE: "mock",
        VITE_TRON_MODEL_DATA_SOURCE: "mock",
      },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  frontendProcess.stdout.on("data", appendFrontendLog);
  frontendProcess.stderr.on("data", appendFrontendLog);

  await waitForFrontend(frontendBaseUrl);
});

test.afterAll(async () => {
  await stopFrontend();
});

test("AC-AGUI-01/02/03: chat shows grouped user AG-UI cards while debug cards are off", async ({ context, page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin: frontendBaseUrl });
  await routeAuthApi(page);
  await page.addInitScript((snapshot) => {
    window.localStorage.setItem("tron-app-shell", JSON.stringify({
      state: {
        openSections: { Core: true },
        selectedPrimarySpaceId: "ENTERPRISE",
        selectedSecondarySpaceId: "TRON_CORE",
        sidebarCollapsed: false,
        workspaceSwitcherSnapshot: snapshot,
      },
      version: 4,
    }));
  }, getWorkspaceSwitcherMock());
  await signIn(page, "/settings/general");

  const eventCardVisibility = page.getByLabel("AG-UI event card visibility");
  await expect(eventCardVisibility.locator("option", { hasText: "Core cards" })).toHaveCount(0);
  await eventCardVisibility.selectOption("off");
  await expect(page.getByText("Hide low-level AG-UI debug event cards")).toBeVisible();

  await page.goto(`${frontendBaseUrl}/chats`);
  const workspaceTrigger = page.locator("#tron-workspace-trigger");
  await expect(workspaceTrigger).toHaveAttribute("aria-busy", "false");
  await workspaceTrigger.click();
  const workspaceSelector = page.locator("#tron-workspace-selector-panel");
  await expect(workspaceSelector).toBeVisible();
  const expandEnterprise = workspaceSelector.getByRole("button", { name: /Expand Enterprise$/ });
  if (await expandEnterprise.count()) await expandEnterprise.click();
  const expandPlatform = workspaceSelector.getByRole("button", { name: /Expand .*Platform$/ });
  if (await expandPlatform.count()) await expandPlatform.click();
  await workspaceSelector.getByRole("button", { name: /Tron Core/ }).click();
  await expect(workspaceTrigger).toContainText("Enterprise/Platform/Tron Core");
  await page.getByRole("button", { name: "New session" }).first().click();
  await page.getByPlaceholder("Enter to send, Shift+Enter for newline").fill("show grouped agui cards");
  await page.getByPlaceholder("Enter to send, Shift+Enter for newline").press("Enter");
  await expect(page.getByLabel("Waiting for model")).toContainText("Waiting for model...");
  const streamingRunWork = page.getByLabel("AG-UI run work");
  const streamingRunSummary = streamingRunWork.getByRole("button", { name: /Worked for/ });
  await expect(streamingRunSummary).toBeVisible();
  await expect(streamingRunSummary).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByLabel("Agent work rail")).toHaveCount(0);

  const transcript = page.getByLabel("Chat transcript");
  const userMessageBubble = transcript.locator("article").first();
  await expect(userMessageBubble).toBeVisible();
  await expect(userMessageBubble).toContainText("show grouped agui cards");
  await expect(userMessageBubble.getByRole("button", { name: "Copy raw markdown" })).toBeVisible();
  await expect(userMessageBubble.getByRole("button", { name: "Edit message" })).toBeVisible();
  await userMessageBubble.getByRole("button", { name: "Copy raw markdown" }).click();
  await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toContain("show grouped agui cards");
  await userMessageBubble.getByRole("button", { name: "Edit message" }).click();
  await expect(page.getByPlaceholder("Enter to send, Shift+Enter for newline")).toHaveValue("show grouped agui cards");

  const runWork = page.getByLabel("AG-UI run work");
  const runSummary = runWork.getByRole("button", { name: /Worked for/ });
  await expect(runSummary).toBeVisible();
  await expect(runSummary).toHaveCount(1);
  await expect(page.getByLabel("Waiting for model")).toHaveCount(0);
  const assistantMessageBubble = transcript.locator("article").filter({ hasText: "Assistant" }).last();
  const copyMarkdownButton = assistantMessageBubble.getByRole("button", { name: /Copy raw markdown/ });
  await expect(copyMarkdownButton).toBeVisible();
  await copyMarkdownButton.click();
  await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toContain("**Recorded in");
  await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toContain("show grouped agui cards");
  await expect(runSummary).toHaveAttribute("aria-expanded", "false");
  await expect(runSummary).not.toContainText(/m/);
  await expect(page.getByLabel("Agent work rail")).toHaveCount(0);
  await runSummary.click();
  await expect(runSummary).toHaveAttribute("aria-expanded", "true");

  const userCards = page.getByLabel("Agent work rail");
  await expect(userCards).toBeVisible();
  await expect(page.getByLabel("AG-UI debug event cards")).toHaveCount(0);

  const thinkingCards = userCards.getByLabel("Thinking");
  await expect(thinkingCards).toHaveCount(2);
  await expect(userCards.getByText("Reasoning captured")).toHaveCount(0);
  await expect(userCards.getByText("This stale run should not render.")).toHaveCount(0);
  const firstThinkingCard = thinkingCards.nth(0);
  const secondThinkingCard = thinkingCards.nth(1);
  const firstThinkingButton = firstThinkingCard.getByRole("button", { name: /Reviewing session context and available skills/ });
  const secondThinkingButton = secondThinkingCard.getByRole("button", { name: /Summarizing tool results before replying/ });
  await expect(firstThinkingButton).toHaveAttribute("aria-expanded", "false");
  await expect(secondThinkingButton).toHaveAttribute("aria-expanded", "false");
  await expect(firstThinkingCard.getByText(/Thought for/)).toHaveCount(0);
  await expect(firstThinkingCard.getByText("Reviewing session context and available skills.")).toBeVisible();
  await expect(userCards.getByText(/Raw reasoning stream segment/)).toHaveCount(0);
  await expect(secondThinkingCard.getByText("Summarizing tool results before replying.")).toBeVisible();
  await firstThinkingButton.click();
  await expect(firstThinkingButton).toHaveAttribute("aria-expanded", "true");
  await expect(firstThinkingCard).toContainText("Reviewing session context and available skills.");
  await expect(firstThinkingCard.getByText(/Raw reasoning stream segment/)).toHaveCount(0);
  await expect(secondThinkingCard.getByText("Reviewing session context and available skills.")).toHaveCount(0);
  const toolRunGroup = userCards.getByLabel("Tool run steps");
  const toolRunGroupButton = toolRunGroup.getByRole("button", { name: /Ran 6 commands/ });
  await expect(toolRunGroupButton).toBeVisible();
  await expect(toolRunGroupButton).toHaveAttribute("aria-expanded", "false");
  await expect(toolRunGroup.getByRole("button", { name: /Ran workspace\.list/ })).toHaveCount(0);
  await toolRunGroupButton.click();
  await expect(toolRunGroupButton).toHaveAttribute("aria-expanded", "true");
  await expect(toolRunGroup.getByRole("button", { name: /Ran workspace\.list/ })).toBeVisible();
  await expect(toolRunGroup.getByRole("button", { name: /Ran variable\.list/ })).toBeVisible();
  await expect(toolRunGroup.getByRole("button", { name: /Ran / })).toHaveCount(7);
  await secondThinkingButton.click();
  await expect(secondThinkingButton).toHaveAttribute("aria-expanded", "true");
  await expect(secondThinkingCard).toContainText("Summarizing tool results before replying.");
  await expect(secondThinkingCard.getByText(/Raw reasoning stream segment/)).toHaveCount(0);

  const toolCard = toolRunGroup.getByRole("button", { name: /Ran workspace\.list/ });
  await expect(toolCard).toBeVisible();
  await expect(firstThinkingCard.getByText(/workspace\.list · .*tool-/)).toHaveCount(0);
  await toolCard.click();
  const parametersCopyButton = toolRunGroup.getByRole("button", { name: "Copy parameters" });
  const resultCopyButton = toolRunGroup.getByRole("button", { name: "Copy result" });
  await expect(parametersCopyButton).toBeVisible();
  await expect(resultCopyButton).toBeVisible();
  const parametersToggle = toolRunGroup.getByRole("button", { name: "Parameters", exact: true });
  await expect(parametersToggle).toHaveAttribute("aria-expanded", "false");
  await expect(toolRunGroup.locator("pre").filter({ hasText: "{}" })).toHaveCount(0);
  await parametersCopyButton.click();
  await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toBe("{}");
  await resultCopyButton.click();
  await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toContain("Listed workspaces");
  await parametersToggle.click();
  await expect(parametersToggle).toHaveAttribute("aria-expanded", "true");
  const parametersBlock = toolRunGroup.locator("pre").filter({ hasText: "{}" });
  await expect(parametersBlock).toBeVisible();
  await expect.poll(async () => parametersBlock.evaluate((node) => window.getComputedStyle(node).backgroundColor)).not.toBe("rgb(255, 255, 255)");
  await expect(toolRunGroup.getByText("Listed workspaces")).toBeVisible();
  await expect(toolRunGroup.getByText("This log belongs elsewhere")).toHaveCount(0);
  await expect.poll(async () => {
    const transcriptText = await transcript.innerText();
    return transcriptText.indexOf("Reviewing session context and available skills.") < transcriptText.indexOf("Assistant");
  }).toBe(true);

  await page.getByPlaceholder("Enter to send, Shift+Enter for newline").fill("show second reasoning stream");
  await page.getByPlaceholder("Enter to send, Shift+Enter for newline").press("Enter");
  const secondRunWork = page.getByLabel("AG-UI run work").last();
  const secondRunSummary = secondRunWork.getByRole("button", { name: /Worked for/ });
  await expect(secondRunSummary).toHaveAttribute("aria-expanded", "false");
  await expect.poll(async () => transcript.evaluate((node) => Math.round(node.scrollHeight - node.clientHeight - node.scrollTop))).toBeLessThanOrEqual(2);
  await secondRunSummary.click();
  const secondUserCards = secondRunWork.getByLabel("Agent work rail");
  const secondRunThinkingCards = secondUserCards.getByLabel("Thinking");
  await expect(secondRunThinkingCards).toHaveCount(2);
  const secondRunFirstThinkingButton = secondRunThinkingCards.nth(0).getByRole("button", { name: /Reviewing session context and available skills/ });
  const secondRunSecondThinkingButton = secondRunThinkingCards.nth(1).getByRole("button", { name: /Summarizing tool results before replying/ });
  await expect(secondRunFirstThinkingButton).toHaveAttribute("aria-expanded", "false");
  await expect(secondRunThinkingCards.nth(0).getByText("Reviewing session context and available skills.")).toBeVisible();
  await expect(secondUserCards.getByText(/Raw reasoning stream segment/)).toHaveCount(0);
  await expect(secondRunThinkingCards.nth(1).getByText("Reviewing session context and available skills.")).toHaveCount(0);
  await secondRunFirstThinkingButton.click();
  await expect(secondRunThinkingCards.nth(0)).toContainText("Reviewing session context and available skills.");
  await expect(secondRunThinkingCards.nth(0).getByText(/Raw reasoning stream segment/)).toHaveCount(0);
  const secondToolRunGroup = secondUserCards.getByLabel("Tool run steps");
  const secondToolRunGroupButton = secondToolRunGroup.getByRole("button", { name: /Ran 6 commands/ });
  await expect(secondToolRunGroupButton).toHaveAttribute("aria-expanded", "false");
  await secondToolRunGroupButton.click();
  await expect(secondToolRunGroup.getByRole("button", { name: /Ran / })).toHaveCount(7);
  await secondRunSecondThinkingButton.click();
  await expect(secondRunThinkingCards.nth(1)).toContainText("Summarizing tool results before replying.");
  await expect(page.getByText("No reasoning content")).toHaveCount(0);
  await expect(page.getByText("Waiting for reasoning content...")).toHaveCount(0);
  await transcript.evaluate((node) => {
    node.scrollTop = 0;
    node.dispatchEvent(new Event("scroll", { bubbles: true }));
  });
  const jumpToLatest = page.getByRole("button", { name: "Jump to latest message" });
  await expect(jumpToLatest).toBeVisible();
  await jumpToLatest.click();
  await expect.poll(async () => transcript.evaluate((node) => Math.round(node.scrollHeight - node.clientHeight - node.scrollTop))).toBeLessThanOrEqual(2);
  await expect(jumpToLatest).toHaveCount(0);

  await page.getByRole("button", { name: "New session" }).first().click();
  await page.getByPlaceholder("Enter to send, Shift+Enter for newline").fill("show scrolling tool log");
  await page.getByPlaceholder("Enter to send, Shift+Enter for newline").press("Enter");
  const logScrollRunWork = page.getByLabel("AG-UI run work").last();
  const logScrollRunSummary = logScrollRunWork.getByRole("button", { name: /Worked for/ });
  await expect(logScrollRunSummary).toHaveAttribute("aria-expanded", "false");
  await logScrollRunSummary.click();
  const logScrollThinking = logScrollRunWork.getByLabel("Thinking").nth(0);
  await expect(logScrollThinking.getByRole("button", { name: /Reviewing session context and available skills/ })).toHaveAttribute("aria-expanded", "true");
  const logScrollToolGroup = logScrollRunWork.getByLabel("Tool run steps").first();
  await expect(logScrollToolGroup.getByRole("button", { name: /Ran 7 commands/ })).toHaveAttribute("aria-expanded", "true");
  await expect(logScrollToolGroup.getByRole("button", { name: /Running bash\.exec/ })).toHaveAttribute("aria-expanded", "true");
  await expect(logScrollToolGroup.getByText("stream log line 40")).toBeVisible();
  const runningLogContainer = logScrollToolGroup.locator("div.max-h-64").filter({ hasText: "stream log line 40" });
  await expect(runningLogContainer).toHaveCount(1);
  await expect.poll(async () => (
    runningLogContainer.evaluate((node) => Math.round(node.scrollHeight - node.clientHeight - node.scrollTop))
  )).toBeLessThanOrEqual(2);

  await page.goto(`${frontendBaseUrl}/settings/general`);
  await page.getByLabel("AG-UI event card visibility").selectOption("all");
  await page.goto(`${frontendBaseUrl}/chats`);
  await page.getByRole("button", { name: "New session" }).first().click();
  await page.getByPlaceholder("Enter to send, Shift+Enter for newline").fill("show debug and user cards");
  await page.getByPlaceholder("Enter to send, Shift+Enter for newline").press("Enter");

  const debugRunWork = page.getByLabel("AG-UI run work");
  const debugRunSummary = debugRunWork.getByRole("button", { name: /Worked for/ });
  await expect(debugRunSummary).toHaveCount(1);
  await expect(debugRunSummary).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByLabel("Agent work rail")).toHaveCount(0);
  await expect(page.getByLabel("AG-UI debug event cards")).toHaveCount(0);
  await debugRunSummary.click();
  await expect(page.getByLabel("Agent work rail")).toBeVisible();
  await expect(page.getByLabel("AG-UI debug event cards")).toBeVisible();
});
