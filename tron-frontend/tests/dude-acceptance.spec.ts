import { expect, test, type Page } from "@playwright/test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
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
const evidence = new WeakMap<Page, { errors: string[]; messages: string[]; chatRequests: unknown[]; chatResponses: { status: number; url: string; body: string }[] }>();
const providerRequests: unknown[] = [];

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
  const captured = { errors: [] as string[], messages: [] as string[], chatRequests: [] as unknown[], chatResponses: [] as { status: number; url: string; body: string }[] };
  evidence.set(page, captured);
  page.on("pageerror", error => captured.errors.push(error.stack ?? error.message));
  page.on("console", message => { if (message.type() === "error" || message.type() === "warning") captured.messages.push(`${message.type()}: ${message.text()}`); });
  page.on("requestfailed", failed => captured.messages.push(`requestfailed: ${failed.method()} ${failed.url()} ${failed.failure()?.errorText}`));
  page.on("request", outgoing => {
    if (outgoing.method() === "POST" && /\/api\/v1\/(?:chats|agents)\//.test(outgoing.url())) {
      captured.chatRequests.push({ url: outgoing.url(), body: outgoing.postData() });
    }
  });
  page.on("response", response => {
    if (response.status() >= 400) captured.messages.push(`HTTP ${response.status()}: ${response.url()}`);
    if (response.request().method() === "POST" && /\/api\/v1\/(?:chats|agents)\//.test(response.url())) {
      // For a streaming chat this resolves when it closes, including a 200
      // stream that ends with RUN_ERROR before ever contacting the provider.
      void response.text().then(body => captured.chatResponses.push({ status: response.status(), url: response.url(), body: body.slice(-16000) }))
        .catch(error => captured.messages.push(`response body: ${response.url()} ${error}`));
    }
  });
});

test.afterEach(async ({ page }, testInfo) => {
  if (testInfo.status === testInfo.expectedStatus) return;
  const captured = evidence.get(page);
  const snapshot = await page.locator("body").ariaSnapshot({ timeout: 2000 }).catch(error => `snapshot failed: ${error}`);
  const html = await page.content().catch(error => `content failed: ${error}`);
  const diagnosticsPath = testInfo.outputPath("dude-page-diagnostics.json");
  await writeFile(diagnosticsPath, JSON.stringify({
    url: page.url(), ...captured, providerRequests, snapshot, html: html.slice(0, 20000), frontend: frontend?.logs(), backend: backendLogs.slice(-16000),
  }, null, 2));
  await testInfo.attach("dude-page-diagnostics", { contentType: "application/json", path: diagnosticsPath });
  const screenshotPath = testInfo.outputPath("dude-failure-screen.png");
  const screenshot = await page.screenshot({ path: screenshotPath, animations: "disabled", timeout: 3000 }).catch(() => null);
  if (screenshot) await testInfo.attach("dude-failure-screen", { contentType: "image/png", path: screenshotPath });
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
    // Context compression uses a dedicated nonstream request with this exact
    // input contract and expects {summary: nonempty string}, not a chat answer.
    let summaryRequest: { previous_summary: string; history: unknown[]; summary_max_utf8_bytes: number } | undefined;
    try {
      const lastContent = payload.messages?.at(-1)?.content;
      const candidate = typeof lastContent === "string" ? JSON.parse(lastContent) : undefined;
      if (typeof candidate?.previous_summary === "string" && Array.isArray(candidate.history)
        && Number.isInteger(candidate.summary_max_utf8_bytes) && candidate.summary_max_utf8_bytes > 0) {
        summaryRequest = candidate;
      }
    } catch { /* Ordinary chat input is not a summary request. */ }
    providerRequests.push({ path: incoming.url, model: payload.model, stream: payload.stream,
      purpose: summaryRequest ? "conversation_summary" : "chat",
      foreground: input.includes("acceptance-foreground-hold"), background: input.includes("acceptance-background-work"),
      messageCount: payload.messages?.length ?? 0, lastMessage: JSON.stringify(payload.messages?.at(-1) ?? null).slice(0, 2000) });
    if (payload.stream && !summaryRequest && input.includes("acceptance-foreground-hold")) {
      foregroundStarted = true;
      await new Promise<void>(resolveGate => { releaseForeground = resolveGate; });
    }
    const summary = input.includes("dude-background-finished")
      ? "The Dude completed task acceptance-background-work with result dude-background-finished."
      : input.includes("dude-foreground-finished")
        ? "The user discussed acceptance-foreground-hold. The Dude replied dude-foreground-finished."
        : summaryRequest?.previous_summary || "No completed acceptance task was recorded.";
    // These fixture summaries are ASCII, so slicing characters also enforces the
    // framework's UTF-8 byte bound without splitting a multibyte character.
    const answer = summaryRequest
      ? JSON.stringify({ summary: summary.slice(0, summaryRequest.summary_max_utf8_bytes) })
      : input.includes("acceptance-background-work") ? "dude-background-finished" : input.includes("acceptance-foreground-hold") ? "dude-foreground-finished" : "Dude acceptance summary";
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
  test.setTimeout(180_000);
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
  await input.fill("acceptance-foreground-hold: continue discussing this with me");
  const sentRequest = page.waitForRequest(outgoing => outgoing.method() === "POST"
    && new URL(outgoing.url()).pathname === `/api/v1/chats/sessions/${session.sessionCode}/messages/stream`);
  // History can still be loading while the composer accepts a draft. The Send
  // button waits for session readiness; an immediate Enter can be ignored then.
  await page.getByRole("button", { name: "Send", exact: true }).click();
  const sent = await sentRequest;
  const traceparent = sent.headers()["traceparent"];
  expect(traceparent).toMatch(/^00-[0-9a-f]{32}-[0-9a-f]{16}-01$/);
  expect(sent.headers()["x-dude-code"]).toBe(session.ownerDudeCode);
  const [, foregroundTrace, browserSpanId] = traceparent.split("-");
  try {
    // Native startup includes persisted context and resource preparation. The
    // controlled provider stays held throughout background task completion.
    await expect.poll(() => foregroundStarted, { timeout: 60_000 }).toBe(true);
    await request("POST", "/api/v1/tasks", { workspaceCode: null, ownerDudeCode: session.ownerDudeCode, title: "acceptance-background-work", description: "acceptance-background-work: finish this while we are still chatting", dueAt: null, priority: "Medium", taskType: "General", sourceType: "Internal", assigneeUserCodes: [], agentCodes: [], appCodes: [] });
    await expect.poll(async () => {
      const events = await request("GET", `/api/v1/dudes/${session.ownerDudeCode}/events`);
      const completed = events.find((event: { source: string; status: string; result?: string }) => event.source === "TaskReminder" && event.status === "Completed" && event.result?.includes("dude-background-finished"));
      backgroundSession = completed?.sessionCode ?? "";
      return Boolean(completed);
    }, { timeout: 60_000 }).toBe(true);
    await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).not.toContainText("dude-background-finished");
    await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).not.toContainText("dude-foreground-finished");
  } finally { releaseForeground(); }
  await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).toContainText("dude-foreground-finished");
  // Query actual local telemetry exported by the browser and native runtime.
  // A trace header alone would not prove that SDK-owned tasks kept the context.
  let foregroundDetail: any;
  await expect.poll(async () => {
    try {
      foregroundDetail = await request("GET", `/api/v1/observability/traces/${foregroundTrace}`);
      const spans = foregroundDetail.spans;
      return spans.some((span: any) => span.spanId === browserSpanId && span.attributes["service.name"] === "tron-frontend")
        && spans.some((span: any) => span.parentSpanId === browserSpanId && span.attributes["http.request.method"] === "POST")
        && spans.some((span: any) => span.name === "tron.session.run")
        && spans.some((span: any) => span.name === "agent.model");
    } catch { return false; }
  }, { timeout: 30_000 }).toBe(true);
  expect(foregroundDetail.trace.sessionCode).toBe(session.sessionCode);
  expect(foregroundDetail.trace.workspaceCode).toBe("");
  expect(foregroundDetail.spans.find((span: any) => span.name === "tron.session.run").attributes["tron.dude.code"]).toBe(session.ownerDudeCode);
  let backgroundDetail: any;
  await expect.poll(async () => {
    const traces = await request("POST", "/api/v1/observability/traces/query", { sessionCode: backgroundSession, current: 1, size: 15 });
    for (const trace of traces.records) {
      const detail = await request("GET", `/api/v1/observability/traces/${trace.traceId}`);
      if (detail.spans.some((span: any) => span.name === "tron.dude.execute") && detail.spans.some((span: any) => span.name === "agent.model")) {
        backgroundDetail = detail; return true;
      }
    }
    return false;
  }, { timeout: 30_000 }).toBe(true);
  expect(backgroundDetail.trace.traceId).not.toBe(foregroundTrace);
  const eventSpan = backgroundDetail.spans.find((span: any) => span.name === "tron.dude.execute");
  expect(eventSpan.parentSpanId).toBeNull();
  expect(eventSpan.attributes["tron.dude.event.source"]).toBe("TaskReminder");
  expect(eventSpan.attributes["tron.dude.code"]).toBe(session.ownerDudeCode);
  expect(backgroundDetail.spans.find((span: any) => span.name === "tron.session.run").parentSpanId).toBe(eventSpan.spanId);
  await writeFile(testInfo.outputPath("dude-traces.json"), JSON.stringify({ foreground: foregroundDetail, background: backgroundDetail }, null, 2));
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
  const traceLink = page.getByRole("link", { name: "View traces", exact: true });
  await expect(traceLink).toHaveAttribute("href", `/settings/observability/traces?sessionCode=${backgroundSession}`);
  expect(errors).toEqual([]);
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("dude-notification-work-history.png") });
  const traceQuery = page.waitForResponse(response => response.request().method() === "POST"
    && new URL(response.url()).pathname === "/api/v1/observability/traces/query"
    && response.request().postDataJSON()?.sessionCode === backgroundSession);
  await traceLink.click();
  const queryResponse = await traceQuery;
  expect(queryResponse.request().postDataJSON().workspaceCode).toBeNull();
  expect((await queryResponse.json()).data.records.some((trace: any) => trace.traceId === backgroundDetail.trace.traceId)).toBe(true);
  await expect(page.getByRole("button", { name: backgroundDetail.trace.traceId, exact: true })).toBeVisible();
  await page.getByRole("button", { name: backgroundDetail.trace.traceId, exact: true }).click();
  await expect(page.getByRole("button", { name: /^tron\.dude\.execute\s/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /^agent\.model\s/ })).toBeVisible();
  await expect(page.getByRole("img", { name: "Waterfall", exact: true }).locator("canvas")).toBeVisible();
  await page.getByRole("button", { name: /^tron\.dude\.execute\s/ }).click();
  await expect(page.getByText("tron.dude.event.source", { exact: true })).toBeVisible();
  await page.screenshot({ animations: "disabled", path: testInfo.outputPath("dude-trace-explorer.png") });
  expect(errors).toEqual([]);
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
