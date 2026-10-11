import { expect, test, type Page } from "@playwright/test";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer as createHttpServer, type Server } from "node:http";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnManagedProcess, stopManagedProcess, type ManagedProcess } from "./support/managed-process";
import { seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let backend: ManagedProcess | undefined;
let frontend: ResourceUiFrontend | undefined;
let provider: Server | undefined;
let directory = "";
let api = "";
let foregroundStarted = false;
let releaseForeground = () => {};
let backendLogs = "";
const evidence = new WeakMap<Page, { errors: string[]; messages: string[] }>();

// Match common::app_with_state_setup rather than the resource-only shell's
// USR-RESOURCE-0001/resource-ui-token identity. Product endpoints remain real.
async function routeAcceptanceProfile(page: Page) {
  const profile = { profileId: "PROFILE-ACCEPTANCE", personAiUserId: "USR-AUTH-0001",
    personAiUsername: "profile.operator", status: "ACTIVE", createdAt: "2026-09-27T00:00:00Z",
    lastActivatedAt: null, avatarUrl: null };
  await page.route(/\/api\/v1\/(?:bootstrap\/status|profiles(?:\/active\/token|\/events)?)(?:\?.*)?$/, async route => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/v1/profiles/events") return route.fulfill({ status: 204 });
    const data = path === "/api/v1/bootstrap/status"
      ? { hasProfiles: true, activeProfileId: profile.profileId, activeProfile: profile, avatarUrl: null, account: null }
      : path === "/api/v1/profiles" ? [profile] : { token: "profile-test-token" };
    await route.fulfill({ json: { code: "SUCCESS", data, message: "ok" } });
  });
}

test.beforeEach(async ({ page }) => {
  const captured = { errors: [] as string[], messages: [] as string[] };
  evidence.set(page, captured);
  page.on("pageerror", error => captured.errors.push(error.stack ?? error.message));
  page.on("console", message => { if (message.type() === "error" || message.type() === "warning") captured.messages.push(`${message.type()}: ${message.text()}`); });
  page.on("requestfailed", failed => captured.messages.push(`requestfailed: ${failed.method()} ${failed.url()} ${failed.failure()?.errorText}`));
  page.on("response", response => { if (response.status() >= 400) captured.messages.push(`HTTP ${response.status()}: ${response.url()}`); });
});

test.afterEach(async ({ page }, testInfo) => {
  if (testInfo.status === testInfo.expectedStatus) return;
  const captured = evidence.get(page);
  const snapshot = await page.locator("body").ariaSnapshot({ timeout: 2000 }).catch(error => `snapshot failed: ${error}`);
  const html = await page.content().catch(error => `content failed: ${error}`);
  await testInfo.attach("dude-page-diagnostics", { contentType: "application/json", body: JSON.stringify({
    url: page.url(), ...captured, snapshot, html: html.slice(0, 20000), frontend: frontend?.logs(), backend: backendLogs.slice(-10000),
  }, null, 2) });
  const screenshot = await page.screenshot({ animations: "disabled", timeout: 3000 }).catch(() => null);
  if (screenshot) await testInfo.attach("dude-failure-screen", { contentType: "image/png", body: screenshot });
});

async function request(method: string, path: string, body?: unknown) {
  const response = await fetch(`${api}${path}`, { method, headers: { "Content-Type": "application/json", Authorization: "Bearer profile-test-token" }, body: body === undefined ? undefined : JSON.stringify(body) });
  const payload = await response.json();
  if (!response.ok) throw new Error(`${method} ${path}: ${JSON.stringify(payload)}`);
  return payload.data;
}

test.beforeAll(async () => {
  test.setTimeout(300_000);
  directory = await mkdtemp(join(tmpdir(), "tron-dude-acceptance-"));
  provider = createHttpServer(async (incoming, response) => {
    let body = "";
    for await (const chunk of incoming) body += chunk;
    const payload = JSON.parse(body);
    const input = JSON.stringify(payload.messages ?? []);
    if (payload.stream && input.includes("acceptance-foreground-hold")) {
      foregroundStarted = true;
      await new Promise<void>(resolveGate => { releaseForeground = resolveGate; });
    }
    const answer = input.includes("acceptance-background-work") ? "dude-background-finished" : input.includes("acceptance-foreground-hold") ? "dude-foreground-finished" : "Dude acceptance summary";
    const usage = { prompt_tokens: 120, completion_tokens: 20, total_tokens: 140 };
    if (payload.stream) {
      response.writeHead(200, { "Content-Type": "text/event-stream" });
      response.write(`data: ${JSON.stringify({ id: "dude-acceptance", choices: [{ index: 0, delta: { role: "assistant", content: answer }, finish_reason: null }] })}\n\n`);
      response.end(`data: ${JSON.stringify({ id: "dude-acceptance", choices: [{ index: 0, delta: {}, finish_reason: "stop" }], usage })}\n\ndata: [DONE]\n\n`);
    } else {
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ id: "dude-acceptance", choices: [{ index: 0, message: { role: "assistant", content: answer }, finish_reason: "stop" }], usage }));
    }
  });
  await new Promise<void>(ready => provider!.listen(0, "127.0.0.1", ready));
  const providerAddress = provider.address();
  if (!providerAddress || typeof providerAddress === "string") throw new Error("provider address missing");
  const port = await new Promise<number>((resolvePort, reject) => {
    const server = createServer(); server.on("error", reject); server.listen(0, "127.0.0.1", () => {
      const address = server.address(); if (!address || typeof address === "string") throw new Error("acceptance port missing");
      server.close(() => resolvePort(address.port));
    });
  });
  api = `http://127.0.0.1:${port}`;
  backendLogs = "";
  backend = spawnManagedProcess("rtk", ["make", "be-ui-acceptance"], {
    cwd: resolve(fileURLToPath(import.meta.url), "../../.."),
    env: { ...process.env, CARGO_BUILD_JOBS: "2", TRON_ACCEPTANCE_DIRECTORY: directory, TRON_ACCEPTANCE_PORT: String(port) }, stdio: "pipe",
  });
  backend.stdout.on("data", data => backendLogs += data.toString()); backend.stderr.on("data", data => backendLogs += data.toString());
  const deadline = Date.now() + 270_000;
  let ready = false;
  while (Date.now() < deadline) {
    if (backend.exitCode !== null) throw new Error(backendLogs);
    try { ready = (await fetch(`${api}/api/v1/health`)).ok; } catch { /* backend starting */ }
    if (ready) break;
    await new Promise(resolveDelay => setTimeout(resolveDelay, 250));
  }
  if (!ready) throw new Error(backendLogs);
  // Configure an executable loop through the same public management API as an
  // existing user. This fixture tests parallel execution, not bootstrap policy.
  await request("POST", "/api/v1/agents/loops", {
    loopCode: "dude-acceptance-react", name: "Dude acceptance React", kind: "React",
    nodes: [], edges: [], maxParallelNodes: 1,
  });
  const main = (await request("GET", "/api/v1/dudes")).find((dude: { isMain: boolean }) => dude.isMain);
  if (!main) throw new Error("default main Dude missing");
  await request("PATCH", `/api/v1/dudes/${main.dudeCode}`, { loopCode: "dude-acceptance-react" });
  await request("POST", "/api/v1/admin/platform/models", {
    displayName: "Dude Acceptance", vendor: "OpenAI", protocol: "OPENAI_COMPATIBLE", modelType: "TEXT", modelCapabilities: ["TEXT_GENERATION"],
    baseUrl: `http://127.0.0.1:${providerAddress.port}/v1`, apiPath: "/chat/completions", apiKeyRef: "synthetic-acceptance", modelName: "acceptance-model", enabled: true,
    thinkingEnabled: false, structuredOutputEnabled: false, temperature: 0.2, maxTokens: 1024, contextWindowTokens: 128000, topP: 1, maxRetries: 0,
  });
  frontend = await startResourceUiFrontend("/dudes", {
    VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_CHAT_DATA_SOURCE: "api", VITE_TRON_MODEL_DATA_SOURCE: "api", VITE_TRON_AGENT_DATA_SOURCE: "api",
    VITE_TRON_API_BASE_URL: "", TRON_FRONTEND_API_PROXY_TARGET: api,
  });
});

test.afterAll(async () => {
  releaseForeground();
  await stopResourceUiFrontend(frontend); await stopManagedProcess(backend);
  await new Promise<void>(closed => provider ? provider.close(() => closed()) : closed());
  if (directory) await rm(directory, { recursive: true, force: true });
});

test("robot opens the persistent platform conversation while task work runs in parallel", async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  const errors = evidence.get(page)!.errors;
  // The robot belongs to the main application shell. Notifications provides
  // that shell without a second chat composer competing with the robot's one.
  await page.goto(`${frontend!.baseUrl}/notifications`);
  await expect(page.getByRole("button", { name: "Platform main Dude conversation", exact: true }),
    `Robot unavailable. Page errors: ${JSON.stringify(errors)}. Frontend: ${frontend!.logs()}`)
    .toBeVisible({ timeout: 10_000 });
  const resolveResponse = page.waitForResponse(response => response.url().endsWith("/api/v1/dudes/main/session") && response.request().method() === "POST");
  await page.getByRole("button", { name: "Platform main Dude conversation", exact: true }).click();
  const session = (await (await resolveResponse).json()).data;
  expect(session.workspaceCode).toBeNull(); expect(session.sessionCode).toBeTruthy();
  const input = page.locator('[data-theme-part="chat-input"]');
  let backgroundSession = "";
  await input.fill("acceptance-foreground-hold: continue discussing this with me"); await input.press("Enter");
  try {
    await expect.poll(() => foregroundStarted).toBe(true);
    await request("POST", "/api/v1/tasks", { workspaceCode: null, ownerDudeCode: session.ownerDudeCode, title: "acceptance-background-work", description: "acceptance-background-work: finish this while we are still chatting", dueAt: null, priority: "Medium", taskType: "General", sourceType: "Internal", assigneeUserCodes: [], agentCodes: [], appCodes: [] });
    await expect.poll(async () => {
      const events = await request("GET", `/api/v1/dudes/${session.ownerDudeCode}/events`);
      const completed = events.find((event: { source: string; status: string; result?: string }) => event.source === "TaskReminder" && event.status === "Completed" && event.result?.includes("dude-background-finished"));
      backgroundSession = completed?.sessionCode ?? "";
      return Boolean(completed);
    }, { timeout: 60_000 }).toBe(true);
    await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).not.toContainText("dude-background-finished");
  } finally { releaseForeground(); }
  await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).toContainText("dude-foreground-finished");
  await page.getByRole("button", { name: "Platform main Dude conversation", exact: true }).click();
  const secondResolve = page.waitForResponse(response => response.url().endsWith("/api/v1/dudes/main/session") && response.request().method() === "POST");
  await page.getByRole("button", { name: "Platform main Dude conversation", exact: true }).click();
  expect((await (await secondResolve).json()).data.sessionCode).toBe(session.sessionCode);
  await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).toContainText("dude-foreground-finished");
  expect(errors).toEqual([]);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("platform-dude-parallel-work.png") });
  await page.getByRole("button", { name: "Platform main Dude conversation", exact: true }).click();
  await page.goto(`${frontend!.baseUrl}/notifications`);
  const notice = page.locator("article").filter({ hasText: "dude-background-finished" });
  await notice.getByRole("button", { name: "Mark read", exact: true }).click();
  await expect(notice.getByRole("button", { name: "Mark unread", exact: true })).toBeVisible();
  await page.reload();
  await expect(notice.getByRole("button", { name: "Mark unread", exact: true })).toBeVisible();
  await notice.locator('a[href*="/dudes?"]').click();
  await expect(page).toHaveURL(new RegExp(`session=${backgroundSession}`));
  const workHistory = page.getByRole("region", { name: "Chat transcript", exact: true });
  await expect(workHistory).toContainText("dude-background-finished");
  await expect(workHistory).not.toContainText("dude-foreground-finished");
  expect(errors).toEqual([]);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("dude-notification-work-history.png") });
});

test("Dude creation stays in agent management with a clear consumption notice", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await seedResourceUiShell(page, "en-US"); await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/settings/agents?tab=dudes`);
  await expect(page.getByRole("button", { name: "New Dude", exact: true }),
    `Dude manager unavailable. Page errors: ${JSON.stringify(evidence.get(page)!.errors)}. Frontend: ${frontend!.logs()}`)
    .toBeVisible({ timeout: 10_000 });
  await page.getByRole("button", { name: "New Dude", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText(/points|credits|consumption/i);
  await expect(dialog).not.toContainText(/Points budget|Session strategy|Notification preferences|Event subscriptions/i);
  await expect(dialog.getByRole("switch", { name: "Background operation", exact: true })).toBeVisible();
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("dude-creation-consumption-notice.png") });
});
