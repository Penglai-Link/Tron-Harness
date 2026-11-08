import { expect, test } from "@playwright/test";
import type { SessionHistory } from "@personai/agent-ui-chat-core";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/awakening"); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

test("onboarding accepts a direct name and custom personality through a restored A2UI form", async ({ page }) => {
  let started = false, ready = false;
  const time = Date.now() - 10_000;
  const messages = [
    { version: "v0.9.1", createSurface: { surfaceId: "dude-soul-setup", catalogId: "https://eliteforge.dev/a2ui/v0.9.1/agent" } },
    { version: "v0.9.1", updateComponents: { surfaceId: "dude-soul-setup", components: [
      { id: "root", component: "Column", children: ["name", "personality", "custom", "submit"] },
      { id: "name", component: "TextField", label: "Name your Dude", value: { path: "/response/name" } },
      { id: "personality", component: "Select", label: "Personality", value: { path: "/response/personality" }, options: [
        { label: "Dry wit — understated warmth", value: "wit" }, { label: "Quiet curiosity — attentive and perceptive", value: "quiet" },
        { label: "Warm independence — caring and candid", value: "warm" }, { label: "Bright energy — expressive and lively", value: "bright" },
      ] },
      { id: "custom", component: "TextField", label: "Custom personality", value: { path: "/response/customPersonality" } },
      { id: "submit", component: "Button", child: "submit-label", action: { event: { name: "submit", context: { response: { path: "/response" } } } } },
      { id: "submit-label", component: "Text", text: "Confirm" },
    ] } },
    { version: "v0.9.1", updateDataModel: { surfaceId: "dude-soul-setup", value: { response: { name: "", personality: "", customPersonality: "" } } } },
  ];
  const interaction = { id: "form", runId: "native", invocationId: null, prompt: [{ type: "Text", text: "Complete the form" }],
    responseSchema: { type: "object" }, presentation: { kind: "a2ui", messages }, response: null as unknown, expiresAtMs: null };
  const history: SessionHistory = { sessionId: "awakening", version: "1", nextCursor: null, pendingInteractions: [interaction], activeRuns: [],
    runs: [{ id: "native", sessionId: "awakening", rootRunId: "native", parentRunId: null, status: "Waiting", startedAtMs: time, finishedAtMs: time + 100, error: null }],
    records: { steps: [], interactions: [interaction], toolInvocations: [], messages: [
      { id: "wake", runId: "native", stepId: null, role: "User", content: [{ type: "Text", text: "Wake up" }], structured: null, status: "Completed", createdAtMs: time },
    ] } };
  const session = { id: 1, sessionCode: "awakening", workspaceCode: "", agentCode: "main-dude", title: "Awakening", status: "ACTIVE", createdAt: time, updatedAt: time, purpose: "Main", ownerDudeCode: "main-dude" };
  await seedResourceUiShell(page);
  await routeResourceUiShellApi(page);
  await page.addInitScript(() => localStorage.setItem("tron.motion.mode", "off"));
  await page.route("**/api/v1/**", async route => {
    const path = new URL(route.request().url()).pathname;
    const json = (data: unknown) => route.fulfill({ json: { code: "SUCCESS", message: "ok", data } });
    if (path === "/api/v1/dudes/main/awakening") {
      if (route.request().method() === "POST") {
        expect(route.request().postDataJSON().a2ui.catalogIds).toContain("https://eliteforge.dev/a2ui/v0.9.1/agent"); started = true;
      }
      return json({ dudeCode: "main-dude", name: ready ? "River" : "Unnamed", state: ready ? "Ready" : started ? "AwaitingUser" : "Uninitialized", started, deferred: false, canResume: started, sessionCode: started ? "awakening" : null, playEntrance: false });
    }
    if (path === "/api/v1/dudes/main/session") return json(session);
    if (path === "/api/v1/dudes") return json([]);
    if (path === "/api/v1/dudes/main-dude/events") return json([{ eventId: "native", source: "UserMessage", status: "Completed", sessionCode: "awakening", createdAt: time, result: null }]);
    if (path.endsWith("/history/query")) return json(history);
    if (path.endsWith("/get-active-run")) return json({ sessionCode: "awakening", activeRun: null });
    if (path.endsWith("/a2ui-state")) return json({ sessionId: "awakening", revision: 0, surfaces: [] });
    if (path.endsWith("/answers/stream")) {
      expect(route.request().postDataJSON().resume[0]).toMatchObject({ interruptId: "form", payload: { name: "River", personality: "", customPersonality: "Quiet, curious and willing to disagree" } });
      ready = true; history.pendingInteractions = []; interaction.response = { name: "River", personality: "", customPersonality: "Quiet, curious and willing to disagree" };
      history.runs[0] = { ...history.runs[0], status: "Completed" };
      history.records.messages.push({ id: "final", runId: "native", stepId: null, role: "Assistant", content: [{ type: "Text", text: "Saved River" }], structured: null, status: "Completed", createdAtMs: time + 1500 });
      const result = { session: { ...session, createdAt: new Date(time).toISOString(), updatedAt: new Date(time).toISOString() },
        run: { runCode: "product", status: "COMPLETED", finalOutput: "Saved River", stepCount: 1 },
        messages: [{ id: "final", messageCode: "final", sessionCode: "awakening", runCode: "native", role: "ASSISTANT", actor: "Dude", channel: "MAIN_AGENT", content: "Saved River", createdAt: new Date(time + 1500).toISOString() }], userInputRequest: null };
      // Replay the full public message protocol. RUN_FINISHED.result alone is
      // legacy payload; relying on the next history poll races the home redirect.
      const frame = (type: string, extra = {}) => ({type, runId:"product", threadId:"awakening", timestamp:time+1500, metadata:{framework:{runId:"native"}}, ...extra});
      const events = [frame("RUN_STARTED"), frame("TEXT_MESSAGE_START", {messageId:"final", role:"assistant"}),
        frame("TEXT_MESSAGE_CONTENT", {messageId:"final", delta:"Saved River"}), frame("TEXT_MESSAGE_END", {messageId:"final"}),
        frame("RUN_FINISHED", {outcome:{type:"success"}, result})];
      return route.fulfill({ contentType: "text/event-stream", body: events.map(event => `event: ${event.type}\ndata: ${JSON.stringify(event)}\n\n`).join("") });
    }
    return route.fallback();
  });
  await page.goto(`${frontend!.baseUrl}/awakening`);
  await expect(page.getByRole("textbox", { name: "Name your Dude", exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Name your Dude", exact: true })).toHaveValue("");
  await expect(page.getByRole("radio")).toHaveCount(0);
  await page.reload();
  await expect(page.getByRole("textbox", { name: "Name your Dude", exact: true })).toBeVisible();
  await page.getByRole("textbox", { name: "Name your Dude", exact: true }).fill("River");
  await page.getByRole("textbox", { name: "Custom personality", exact: true }).fill("Quiet, curious and willing to disagree");
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(page.getByRole("region", { name: "Chat transcript", exact: true })).toContainText("Saved River");
  await expect(page).toHaveURL(/\/workspace$/, { timeout: 12_000 });
});
