/**
 * Real-browser acceptance for the stable two-tool gateway.
 *
 * The backend under test is `make be-ui-acceptance` (real AppState, real HTTP
 * routes, real PTY) with a local OpenAI-compatible stub model. The model stub
 * scripts OpenAI `tool_calls`, so the assertions below observe the real
 * framework loop: `tools` stability, `tool_search` discovery output, real
 * `tool_execute` execution through `bash_exec`, and the AG-UI card titles the
 * browser actually renders.
 */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer as createHttpServer, type Server } from "node:http";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnManagedProcess, stopManagedProcess, type ManagedProcess } from "./support/managed-process";
import { seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

type StubToolCall = { name: string; args: unknown };
type StubMessage = { role?: string; content?: unknown; tool_call_id?: string; name?: string };
type RecordedRequest = {
  marker: string;
  stream: boolean;
  toolNames: string[];
  toolNamesSorted: string[];
  messages: StubMessage[];
  hasToolResult: boolean;
  raw: unknown;
};

const GATEWAY_TOOL_NAMES = ["tool_execute", "tool_search"];
const MODEL_DISPLAY_NAME = "Tool Gateway Acceptance";
const EXEC_COMMAND = "echo acc-gateway-hello";
const PARALLEL_COMMANDS = ["echo acc-parallel-alpha", "echo acc-parallel-beta"];

type Scenario = { toolCalls: StubToolCall[]; final: string };

/**
 * Marker -> scripted turn. The first model call of a marker replies with the
 * scripted `tool_calls`; every later call of the same marker (which carries a
 * `role:"tool"` result) replies with `final`. That keeps the stub stateless
 * across tests, so scenario order cannot leak into the transcript.
 */
const SCENARIOS: Record<string, Scenario> = {
  "gateway-baseline": { toolCalls: [], final: "gateway-baseline-reply" },
  "gateway-discovery": {
    toolCalls: [{ name: "tool_search", args: { domainCode: "bash", limit: 5 } }],
    final: "gateway-discovery-reply",
  },
  "gateway-execute": {
    toolCalls: [{ name: "tool_execute", args: { tool: "bash_exec", arguments: { command: EXEC_COMMAND } } }],
    final: "gateway-execute-reply",
  },
  "gateway-parallel": {
    toolCalls: [
      { name: "tool_execute", args: { tool: "bash_exec", arguments: { command: PARALLEL_COMMANDS[0] } } },
      { name: "tool_execute", args: { tool: "bash_exec", arguments: { command: PARALLEL_COMMANDS[1] } } },
    ],
    final: "gateway-parallel-reply",
  },
  "gateway-en-discovery": {
    toolCalls: [{ name: "tool_search", args: { domainCode: "bash", limit: 5 } }],
    final: "gateway-en-discovery-reply",
  },
  "gateway-en-execute": {
    toolCalls: [{ name: "tool_execute", args: { tool: "bash_exec", arguments: { command: EXEC_COMMAND } } }],
    final: "gateway-en-execute-reply",
  },
};

let backend: ManagedProcess | undefined;
let frontend: ResourceUiFrontend | undefined;
let provider: Server | undefined;
let directory = "";
let api = "";
let backendLogs = "";
const requests: RecordedRequest[] = [];

/** Mirrors `tron-backend/tests/common` auth: the acceptance profile and its bearer token. */
async function routeAcceptanceProfile(page: Page) {
  const profile = {
    profileId: "PROFILE-ACCEPTANCE",
    personAiUserId: "USR-AUTH-0001",
    personAiUsername: "profile.operator",
    status: "ACTIVE",
    createdAt: "2026-09-27T00:00:00Z",
    lastActivatedAt: null,
    avatarUrl: null,
  };
  await page.route(/\/api\/v1\/(?:bootstrap\/status|profiles(?:\/active\/token|\/events)?)(?:\?.*)?$/, async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/v1/profiles/events") return route.fulfill({ status: 204 });
    const data = path === "/api/v1/bootstrap/status"
      ? { hasProfiles: true, activeProfileId: profile.profileId, activeProfile: profile, avatarUrl: null, account: null }
      : path === "/api/v1/profiles"
        ? [profile]
        : { token: "profile-test-token" };
    await route.fulfill({ json: { code: "SUCCESS", data, message: "ok" } });
  });
}

function recordRequest(payload: Record<string, unknown>): RecordedRequest {
  const messages = (Array.isArray(payload.messages) ? payload.messages : []) as StubMessage[];
  const tools = Array.isArray(payload.tools) ? payload.tools as Array<{ function?: { name?: string } }> : [];
  const toolNames = tools.map((tool) => tool.function?.name ?? "");
  // The scripted turn follows the *latest* user message: a session that keeps
  // chatting carries earlier markers and earlier tool results in its history.
  const lastUserIndex = messages.map((message) => message.role).lastIndexOf("user");
  const lastUserText = JSON.stringify(messages[lastUserIndex]?.content ?? "");
  const marker = Object.keys(SCENARIOS)
    .sort((left, right) => right.length - left.length)
    .find((key) => lastUserText.includes(key)) ?? "";
  const recorded: RecordedRequest = {
    marker,
    stream: payload.stream === true,
    toolNames,
    toolNamesSorted: [...toolNames].sort(),
    messages,
    hasToolResult: messages.slice(lastUserIndex + 1).some((message) => message.role === "tool"),
    raw: payload,
  };
  requests.push(recorded);
  return recorded;
}

function isSummaryRequest(payload: Record<string, unknown>) {
  const messages = Array.isArray(payload.messages) ? payload.messages as StubMessage[] : [];
  const last = messages.at(-1)?.content;
  if (typeof last !== "string") return false;
  try {
    const parsed = JSON.parse(last) as { previous_summary?: unknown; history?: unknown; summary_max_utf8_bytes?: unknown };
    return typeof parsed.previous_summary === "string"
      && Array.isArray(parsed.history)
      && Number.isInteger(parsed.summary_max_utf8_bytes);
  } catch {
    return false;
  }
}

const usage = { prompt_tokens: 240, completion_tokens: 32, total_tokens: 272 };

function writeTextStream(response: import("node:http").ServerResponse, id: string, answer: string) {
  response.writeHead(200, { "Content-Type": "text/event-stream" });
  response.write(`data: ${JSON.stringify({ id, choices: [{ index: 0, delta: { role: "assistant", content: answer }, finish_reason: null }] })}\n\n`);
  response.end(`data: ${JSON.stringify({ id, choices: [{ index: 0, delta: {}, finish_reason: "stop" }], usage })}\n\ndata: [DONE]\n\n`);
}

function writeToolCallStream(response: import("node:http").ServerResponse, id: string, calls: StubToolCall[]) {
  response.writeHead(200, { "Content-Type": "text/event-stream" });
  response.write(`data: ${JSON.stringify({
    id,
    choices: [{
      index: 0,
      delta: {
        role: "assistant",
        tool_calls: calls.map((call, index) => ({
          index,
          id: `${id}-call-${index + 1}`,
          type: "function",
          function: { name: call.name, arguments: JSON.stringify(call.args) },
        })),
      },
      finish_reason: "tool_calls",
    }],
    usage,
  })}\n\n`);
  response.end("data: [DONE]\n\n");
}

test.beforeAll(async () => {
  test.setTimeout(300_000);
  directory = await mkdtemp(join(tmpdir(), "tron-tool-gateway-"));
  provider = createHttpServer(async (incoming, response) => {
    let body = "";
    for await (const chunk of incoming) body += chunk;
    const payload = JSON.parse(body) as Record<string, unknown>;
    const recorded = recordRequest(payload);
    const summaryRequest = isSummaryRequest(payload);
    if (summaryRequest) {
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ id: "gateway-summary", choices: [{ index: 0, message: { role: "assistant", content: JSON.stringify({ summary: "gateway acceptance summary" }) }, finish_reason: "stop" }], usage }));
      return;
    }
    const scenario = SCENARIOS[recorded.marker];
    const answer = scenario ? scenario.final : "gateway-acceptance-summary";
    if (payload.stream !== true) {
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ id: "gateway-acceptance", choices: [{ index: 0, message: { role: "assistant", content: answer }, finish_reason: "stop" }], usage }));
      return;
    }
    if (scenario && scenario.toolCalls.length > 0 && !recorded.hasToolResult) {
      writeToolCallStream(response, "gateway-acceptance", scenario.toolCalls);
      return;
    }
    writeTextStream(response, "gateway-acceptance", answer);
  });
  await new Promise<void>((ready) => provider!.listen(0, "127.0.0.1", ready));
  const providerAddress = provider.address();
  if (!providerAddress || typeof providerAddress === "string") throw new Error("provider address missing");
  const port = await new Promise<number>((resolvePort, reject) => {
    const server = createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") throw new Error("acceptance port missing");
      server.close(() => resolvePort(address.port));
    });
  });
  api = `http://127.0.0.1:${port}`;
  backendLogs = "";
  backend = spawnManagedProcess("make", ["be-ui-acceptance", `CARGO=${process.env.TRON_ACCEPTANCE_CARGO || "cargo"}`], {
    cwd: resolve(fileURLToPath(import.meta.url), "../../.."),
    env: {
      ...process.env,
      CARGO_NET_OFFLINE: "true",
      CARGO_PROFILE_DEV_DEBUG: process.env.CARGO_PROFILE_DEV_DEBUG || "0",
      CARGO_INCREMENTAL: process.env.CARGO_INCREMENTAL || "0",
      CARGO_BUILD_JOBS: "2",
      TRON_ACCEPTANCE_DIRECTORY: directory,
      TRON_ACCEPTANCE_PORT: String(port),
    },
    stdio: "pipe",
  });
  backend.stdout.on("data", (data: Buffer) => { backendLogs += data.toString(); });
  backend.stderr.on("data", (data: Buffer) => { backendLogs += data.toString(); });
  const deadline = Date.now() + 270_000;
  let ready = false;
  while (Date.now() < deadline) {
    if (backend.exitCode !== null) throw new Error(backendLogs);
    try { ready = (await fetch(`${api}/api/v1/health`)).ok; } catch { /* backend starting */ }
    if (ready) break;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
  }
  if (!ready) throw new Error(backendLogs);

  const agentResponse = await fetch(`${api}/api/v1/agents/workspaces/TRON_CORE/internal`, {
    headers: { Authorization: "Bearer profile-test-token" },
  });
  expect(agentResponse.ok).toBe(true);
  const agents = (await agentResponse.json()).data as Array<{ agentCode: string }>;
  const agentCodes = agents.map((agent) => agent.agentCode);
  logEvidence("workspace.internalAgents", agentCodes);
  // The workspace ships more than one internal agent; the chat page defaults to
  // `main-agent`, which is the one authorized for every platform tool.
  expect(agentCodes, `workspace internal agents: ${JSON.stringify(agentCodes)}`).toContain("main-agent");

  const modelResponse = await fetch(`${api}/api/v1/admin/platform/models`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: "Bearer profile-test-token" },
    body: JSON.stringify({
      displayName: MODEL_DISPLAY_NAME, vendor: "OpenAI", protocol: "OPENAI_COMPATIBLE", modelType: "TEXT", modelCapabilities: ["TEXT_GENERATION"],
      baseUrl: `http://127.0.0.1:${providerAddress.port}/v1`, apiPath: "/chat/completions", apiKeyRef: "sk-acceptance", modelName: "acceptance-model", enabled: true,
      thinkingEnabled: false, structuredOutputEnabled: false, temperature: 0.2, maxTokens: 1024, contextWindowTokens: 128000, topP: 1, maxRetries: 0,
    }),
  });
  expect(modelResponse.ok).toBe(true);
  frontend = await startResourceUiFrontend("/chats", {
    VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_CHAT_DATA_SOURCE: "api", VITE_TRON_MODEL_DATA_SOURCE: "api", VITE_TRON_AGENT_DATA_SOURCE: "api",
    VITE_TRON_API_BASE_URL: "", TRON_FRONTEND_API_PROXY_TARGET: api,
  });
});

test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  await stopManagedProcess(backend);
  await new Promise<void>((closed) => provider ? provider.close(() => closed()) : closed());
  if (directory) await rm(directory, { recursive: true, force: true });
});

/**
 * The shell renders sibling buttons that share the "new chat" accessible name
 * (the sidebar entry and per-project `+`). The sidebar entry is the first in
 * DOM order and stays disabled until the workspace resolves, so wait for that
 * one to enable instead of clicking whichever match Playwright finds first.
 */
async function clickFirstEnabled(page: Page, name: string) {
  const buttons = page.getByRole("button", { name, exact: true });
  await expect(buttons.first()).toBeVisible({ timeout: 30_000 });
  await expect(buttons.first()).toBeEnabled({ timeout: 60_000 });
  await buttons.first().click();
}

async function openChat(page: Page, locale: "zh-CN" | "en-US") {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await seedResourceUiShell(page, locale);
  await routeAcceptanceProfile(page);
  await page.goto(`${frontend!.baseUrl}/chats`);
  await clickFirstEnabled(page, locale === "zh-CN" ? "新会话" : "New chat");
  await clickFirstEnabled(page, locale === "zh-CN" ? "选择模型" : "Select chat model");
  await page.getByRole("menuitemradio", { name: MODEL_DISPLAY_NAME, exact: true }).click();
}

function transcriptOf(page: Page): Locator {
  return page.getByRole("region", { name: "Chat transcript", exact: true });
}

async function send(page: Page, text: string) {
  const input = page.locator('[data-theme-part="chat-input"]');
  await input.fill(text);
  await input.press("Enter");
}

/** The work rail auto-collapses when the run completes; open it to reach the cards. */
async function expandWorkRail(transcript: Locator) {
  const summaries = transcript.locator("button.ef-chat__rail-summary");
  const count = await summaries.count();
  for (let index = 0; index < count; index += 1) {
    const button = summaries.nth(index);
    if ((await button.getAttribute("aria-expanded")) === "false") await button.click();
  }
  const entryToggles = transcript.locator("button.ef-chat__rail-entry-toggle");
  const entryCount = await entryToggles.count();
  for (let index = 0; index < entryCount; index += 1) {
    const button = entryToggles.nth(index);
    if ((await button.getAttribute("aria-expanded")) === "false") await button.click();
  }
}

async function visibleToolTitles(transcript: Locator): Promise<string[]> {
  const titles = transcript.locator(".ef-chat__tool-title");
  await expect(titles.first()).toBeVisible({ timeout: 30_000 });
  const texts: string[] = [];
  const count = await titles.count();
  for (let index = 0; index < count; index += 1) {
    const text = (await titles.nth(index).innerText()).trim();
    if (text) texts.push(text);
  }
  return texts;
}

function requestsFor(marker: string) {
  return requests.filter((request) => request.marker === marker);
}

function toolResultTexts(recorded: RecordedRequest) {
  return recorded.messages
    .filter((message) => message.role === "tool")
    .map((message) => typeof message.content === "string" ? message.content : JSON.stringify(message.content));
}

function expectGatewayToolsOnly(recorded: RecordedRequest[], label: string) {
  expect(recorded.length, `${label}: at least one model request must be observed`).toBeGreaterThan(0);
  // Strict form of the cache-stability invariant: the raw array must be exactly
  // the two gateway call names, already ordered by call_name (discovery output
  // must never widen it).
  const observed = recorded.map((request) => request.toolNames);
  expect(observed, `${label}: every model request must expose exactly ["tool_execute","tool_search"]`).toEqual(
    observed.map(() => GATEWAY_TOOL_NAMES),
  );
}

function logEvidence(label: string, value: unknown) {
  // eslint-disable-next-line no-console
  console.log(`[tool-gateway-evidence] ${label}: ${JSON.stringify(value)}`);
}

test("1. baseline conversation keeps the model-visible tools array stable", async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await openChat(page, "zh-CN");
  const transcript = transcriptOf(page);
  await send(page, "gateway-baseline: 普通对话基线检查");
  await expect(transcript).toContainText("gateway-baseline-reply", { timeout: 60_000 });

  const recorded = requestsFor("gateway-baseline");
  expectGatewayToolsOnly(recorded, "baseline");
  logEvidence("baseline.tools", recorded.map((request) => request.toolNames));
  logEvidence("baseline.toolsSorted", recorded.map((request) => request.toolNamesSorted));
  expect(errors).toEqual([]);
  await page.screenshot({ path: testInfo.outputPath("01-baseline.png"), animations: "disabled" });
});

test("2. tool_search discovery returns the bash domain and renders a localized card", async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  await openChat(page, "zh-CN");
  const transcript = transcriptOf(page);
  await send(page, "gateway-discovery: 找出 bash 领域工具");
  await expect(transcript).toContainText("gateway-discovery-reply", { timeout: 60_000 });

  const recorded = requestsFor("gateway-discovery");
  expectGatewayToolsOnly(recorded, "discovery");
  const followUp = recorded.find((request) => request.hasToolResult);
  expect(followUp, "a follow-up model request carrying the tool result must exist").toBeTruthy();
  const results = toolResultTexts(followUp!);
  expect(results.length).toBeGreaterThan(0);
  const discoveryResult = results.join("\n");
  expect(discoveryResult, "tool_search must return the real bash domain inventory").toContain("bash_exec");
  logEvidence("discovery.tools", recorded.map((request) => request.toolNames));
  logEvidence("discovery.followUpToolResult", discoveryResult.slice(0, 600));

  await expandWorkRail(transcript);
  const titles = await visibleToolTitles(transcript);
  logEvidence("discovery.cardTitles", titles);
  expect(titles.some((title) => title.includes("搜索工具")), `zh discovery card title must be human readable: ${JSON.stringify(titles)}`).toBe(true);
  const transcriptText = (await transcript.innerText());
  logEvidence("discovery.transcriptHasRawToolSearch", transcriptText.includes("tool_search"));
  expect(transcriptText, "zh transcript must not expose the raw tool_search id").not.toContain("tool_search");
  await page.screenshot({ path: testInfo.outputPath("02-discovery.png"), animations: "disabled" });
});

test("3. tool_execute really runs bash_exec and the card hides the raw ids", async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  await openChat(page, "zh-CN");
  const transcript = transcriptOf(page);
  await send(page, "gateway-execute: 执行一条命令");
  await expect(transcript).toContainText("gateway-execute-reply", { timeout: 60_000 });

  const recorded = requestsFor("gateway-execute");
  expectGatewayToolsOnly(recorded, "execute");
  const followUp = recorded.find((request) => request.hasToolResult);
  expect(followUp, "a follow-up model request carrying the tool result must exist").toBeTruthy();
  const executionResult = toolResultTexts(followUp!).join("\n");
  expect(executionResult, "the command really executed in the acceptance backend").toContain("acc-gateway-hello");
  logEvidence("execute.tools", recorded.map((request) => request.toolNames));
  logEvidence("execute.followUpToolResult", executionResult.slice(0, 600));

  await expandWorkRail(transcript);
  const titles = await visibleToolTitles(transcript);
  logEvidence("execute.cardTitles", titles);
  expect(titles.some((title) => title.includes("echo acc-gateway-hello")), `zh execute card title must carry the command: ${JSON.stringify(titles)}`).toBe(true);
  const transcriptText = await transcript.innerText();
  logEvidence("execute.transcriptHasRawIds", { toolExecute: transcriptText.includes("tool_execute"), bashExec: transcriptText.includes("bash_exec") });
  expect(transcriptText, "zh transcript must not expose tool_execute").not.toContain("tool_execute");
  expect(transcriptText, "zh transcript must not expose bash_exec").not.toContain("bash_exec");
  await page.screenshot({ path: testInfo.outputPath("03-execute.png"), animations: "disabled" });
  // The Tron work rail composes command cards from the host i18n
  // `chat.management.tool.commandSummary` =「执行：{{command}}」, so the card
  // title is a localized action summary carrying the command instead of the
  // gateway's raw id. Assert the measured wording (verified against the real
  // browser) and that no raw tool id leaks anywhere in the transcript.
  expect(titles, "zh execute card title must be a localized action summary carrying the command").toContain("执行：echo acc-gateway-hello");
});

test("4. two parallel tool_calls execute in one batch and render two cards", async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  await openChat(page, "zh-CN");
  const transcript = transcriptOf(page);
  await send(page, "gateway-parallel: 并行执行两条命令");
  await expect(transcript).toContainText("gateway-parallel-reply", { timeout: 60_000 });

  const recorded = requestsFor("gateway-parallel");
  expectGatewayToolsOnly(recorded, "parallel");
  const followUp = recorded.find((request) => request.hasToolResult);
  expect(followUp, "a follow-up model request carrying both tool results must exist").toBeTruthy();
  const results = toolResultTexts(followUp!).join("\n");
  expect(results, "the first parallel command must have executed").toContain("acc-parallel-alpha");
  expect(results, "the second parallel command must have executed").toContain("acc-parallel-beta");
  logEvidence("parallel.tools", recorded.map((request) => request.toolNames));
  logEvidence("parallel.followUpToolResult", results.slice(0, 800));

  await expandWorkRail(transcript);
  const titles = await visibleToolTitles(transcript);
  logEvidence("parallel.cardTitles", titles);
  expect(titles.some((title) => title.includes("acc-parallel-alpha")), `parallel card alpha missing: ${JSON.stringify(titles)}`).toBe(true);
  expect(titles.some((title) => title.includes("acc-parallel-beta")), `parallel card beta missing: ${JSON.stringify(titles)}`).toBe(true);
  expect(await transcript.locator("button.ef-chat__tool-toggle").count(), "two tool cards must be rendered").toBe(2);
  const transcriptText = await transcript.innerText();
  // The batch is presented as one grouped work-rail entry (zh copy).
  expect(transcriptText, "the grouped work-rail label must summarize the batch").toContain("执行 2 个命令");
  logEvidence("parallel.transcriptHasRawIds", { toolExecute: transcriptText.includes("tool_execute"), bashExec: transcriptText.includes("bash_exec") });
  expect(transcriptText).not.toContain("tool_execute");
  expect(transcriptText).not.toContain("bash_exec");
  await page.screenshot({ path: testInfo.outputPath("04-parallel.png"), animations: "disabled" });
});

test("5. reverse validation: en-US renders the same gateway cards in English", async ({ page }, testInfo) => {
  test.setTimeout(180_000);
  await openChat(page, "en-US");
  const transcript = transcriptOf(page);

  await send(page, "gateway-en-discovery: list bash tools");
  await expect(transcript).toContainText("gateway-en-discovery-reply", { timeout: 60_000 });
  const discovery = requestsFor("gateway-en-discovery");
  expectGatewayToolsOnly(discovery, "en discovery");
  const discoveryResult = toolResultTexts(discovery.find((request) => request.hasToolResult)!).join("\n");
  expect(discoveryResult).toContain("bash_exec");
  await expandWorkRail(transcript);
  let titles = await visibleToolTitles(transcript);
  logEvidence("en.discoveryCardTitles", titles);
  expect(titles.some((title) => title.includes("Search tools")), `en discovery card title missing: ${JSON.stringify(titles)}`).toBe(true);
  expect(titles.some((title) => title.includes("bash")), `en discovery card title must carry the domain: ${JSON.stringify(titles)}`).toBe(true);
  expect((await transcript.innerText())).not.toContain("tool_search");
  await page.screenshot({ path: testInfo.outputPath("05-en-discovery.png"), animations: "disabled" });

  await send(page, "gateway-en-execute: run a command");
  await expect(transcript).toContainText("gateway-en-execute-reply", { timeout: 60_000 });
  const execute = requestsFor("gateway-en-execute");
  expectGatewayToolsOnly(execute, "en execute");
  const executionResult = toolResultTexts(execute.find((request) => request.hasToolResult)!).join("\n");
  expect(executionResult).toContain("acc-gateway-hello");
  await expandWorkRail(transcript);
  titles = await visibleToolTitles(transcript);
  logEvidence("en.executeCardTitles", titles);
  expect(titles.some((title) => title.includes("echo acc-gateway-hello")), `en execute card title missing: ${JSON.stringify(titles)}`).toBe(true);
  const transcriptText = await transcript.innerText();
  logEvidence("en.transcriptHasRawIds", { toolExecute: transcriptText.includes("tool_execute"), bashExec: transcriptText.includes("bash_exec") });
  expect(transcriptText).not.toContain("tool_execute");
  expect(transcriptText).not.toContain("bash_exec");
  await page.screenshot({ path: testInfo.outputPath("06-en-execute.png"), animations: "disabled" });
  // Same as the zh case: the host i18n `chat.management.tool.commandSummary`
  // renders 「Run: {{command}}」 in en-US. Assert the measured wording rather
  // than a brief-invented template, keeping the raw-name invariant above.
  expect(titles, "en execute card title must be a localized action summary carrying the command").toContain("Run: echo acc-gateway-hello");
});