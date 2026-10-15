import { expect, test } from "@playwright/test";
import type { SessionHistory } from "@eliteforge/agent-ui-chat-core";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/awakening"); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

test("restored awakening preserves reading position and enters home after the answered run completes", async ({ page }) => {
  const time = Date.now() - 10_000;
  let ready = false;
  const interaction = { id: "question", runId: "native", invocationId: null, prompt: [], responseSchema: { type: "object" },
    presentation: { kind: "ask_user", intro: "Choose your Dude's style", questions: [{ id: "style", question: "Which style?", options: [{ id: "direct", label: "Direct" }] }] },
    response: null as unknown, expiresAtMs: null };
  const history: SessionHistory = { sessionId: "awakening", version: "1", nextCursor: null, pendingInteractions: [interaction], activeRuns: [],
    runs: [{ id: "native", sessionId: "awakening", rootRunId: "native", parentRunId: null, status: "Waiting", startedAtMs: time, finishedAtMs: time + 100, error: null }],
    records: { steps: [], interactions: [interaction], messages: [
      { id: "wake", runId: "native", stepId: null, role: "User", content: [{ type: "Text", text: Array.from({ length: 45 }, (_, i) => `Initialization context ${i + 1}`).join("\n") }], structured: null, status: "Completed", createdAtMs: time },
      { id: "ask", runId: "native", stepId: null, role: "Assistant", content: [], structured: null, status: "Completed", createdAtMs: time + 10 },
    ], toolInvocations: [{ id: "ask-tool", callId: "ask-tool", runId: "native", stepId: "ask", messageId: "ask", toolId: "ask", toolName: "chat_ask_user", arguments: {},
      content: [], structured: { frameworkInteractionId: "question" }, status: "Completed", isError: false, resultReference: null, startedAtMs: time + 20, finishedAtMs: time + 100 }] } };
  const session = { id: 1, sessionCode: "awakening", workspaceCode: "", agentCode: "main-dude", title: "Awakening", status: "ACTIVE", createdAt: time, updatedAt: time, purpose: "Main", ownerDudeCode: "main-dude" };
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await seedResourceUiShell(page);
  await routeResourceUiShellApi(page);
  await page.addInitScript(() => localStorage.setItem("tron.motion.mode", "off"));
  await page.route("**/api/v1/**", async route => {
    const path = new URL(route.request().url()).pathname;
    const json = (data: unknown) => route.fulfill({ json: { code: "SUCCESS", message: "ok", data } });
    if (path === "/api/v1/dudes/main/awakening") return json({ dudeCode: "main-dude", name: "Neo", state: ready ? "Ready" : "AwaitingUser", started: true, deferred: false, canResume: true, playEntrance: false });
    if (path === "/api/v1/dudes/main/session") return json(session);
    if (path === "/api/v1/dudes") return json([]);
    if (path === "/api/v1/dudes/main-dude/events") return json([{ eventId: "native", source: "UserMessage", status: "Completed", sessionCode: "awakening", createdAt: time, result: null }]);
    if (path.endsWith("/history/query")) return json(history);
    if (path.endsWith("/get-active-run")) return json({ sessionCode: "awakening", activeRun: null });
    if (path.endsWith("/a2ui-state")) return json({ sessionId: "awakening", revision: 0, surfaces: [] });
    if (path.endsWith("/answers/stream")) {
      expect(route.request().postDataJSON().resume[0].interruptId).toBe("question");
      ready = true;
      history.pendingInteractions = [];
      interaction.response = { answers: { style: { option_id: "direct" } } };
      history.runs[0] = { ...history.runs[0], startedAtMs: time + 1000, finishedAtMs: time + 2000, status: "Completed" };
      history.records.messages.push(
        { id: "resume", runId: "native", stepId: null, role: "Assistant", content: [], structured: null, status: "Completed", createdAtMs: time + 1100 },
        { id: "final", runId: "native", stepId: null, role: "Assistant", content: [{ type: "Text", text: "Soul saved" }], structured: null, status: "Completed", createdAtMs: time + 1500 },
      );
      history.records.toolInvocations.push({ ...history.records.toolInvocations[0], id: "save-tool", callId: "save-tool", stepId: "resume", messageId: "resume", toolName: "dude_soul_update", structured: null, startedAtMs: time + 1200, finishedAtMs: time + 1300 });
      const internal = { ...history.runs[0], id: "internal", rootRunId: "internal", status: "Running" as const, finishedAtMs: null };
      history.runs.push(internal); history.activeRuns = [internal];
      const result = { session: { ...session, createdAt: new Date(time).toISOString(), updatedAt: new Date(time).toISOString() },
        run: { runCode: "product", status: "COMPLETED", finalOutput: "Soul saved", stepCount: 1 },
        messages: [{ id: "final", messageCode: "final", sessionCode: "awakening", runCode: "native", role: "ASSISTANT", actor: "Dude", channel: "MAIN_AGENT", content: "Soul saved", createdAt: new Date(time + 1500).toISOString() }], userInputRequest: null };
      const frame = (type: string, extra = {}) => ({ type, runId: "product", threadId: "awakening", timestamp: time + 1000, metadata: { framework: { runId: "native" } }, ...extra });
      return route.fulfill({ contentType: "text/event-stream", body: [frame("RUN_STARTED"), frame("TOOL_CALL_START", { toolCallId: "save-tool", toolCallName: "dude_soul_update" }),
        frame("RUN_FINISHED", { outcome: { type: "success" }, result })].map(event => `event: ${event.type}\ndata: ${JSON.stringify(event)}\n\n`).join("") });
    }
    return route.fallback();
  });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`${frontend!.baseUrl}/awakening`);
  await expect(page.getByRole("radio", { name: "Direct", exact: true })).toBeVisible();
  const transcript = page.getByRole("region", { name: "Chat transcript", exact: true });
  await transcript.hover();
  await page.mouse.wheel(0, -40);
  await expect.poll(() => transcript.evaluate(el => el.scrollHeight - el.scrollTop - el.clientHeight)).toBeGreaterThan(5);
  const before = await transcript.evaluate(el => el.scrollTop);
  // A history poll changes the content while the user is reading above bottom.
  history.records.messages[0].content.push({ type: "Text", text: "Additional initialization context" });
  await expect(transcript).toContainText("Additional initialization context");
  expect(Math.abs(await transcript.evaluate(el => el.scrollTop) - before)).toBeLessThan(5);
  await expect(page).toHaveURL(/\/awakening$/);
  await page.reload();
  await expect(page.getByRole("radio", { name: "Direct", exact: true })).toBeVisible();
  await page.getByText("Direct", { exact: true }).click();
  await expect(page.getByRole("radio", { name: "Direct", exact: true })).toBeChecked();
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(transcript).toContainText("Soul saved");
  await expect(page.getByRole("button", { name: "Stop", exact: true })).toHaveCount(0);
  await expect(page.locator('[data-history-interaction-id="question"]')).toHaveCount(1);
  await expect(page).toHaveURL(/\/workspace$/, { timeout: 12_000 });
  expect(errors).toEqual([]);
});
