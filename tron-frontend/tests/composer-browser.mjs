/** Playwright CLI acceptance against real Tron routes/Framework and a controlled fixture model.
 * Seed profile bootstrap, grant composer-review, and register Composer Preview before calling.
 */
export async function verifyComposerIntegration(page, { apiBase, modelBase, skillName = "composer-review", modelName = "Composer Preview" }) {
  const checks = [];
  const check = (name, actual, expected = true) => {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(`${name}: ${JSON.stringify(actual)} !== ${JSON.stringify(expected)}`);
    checks.push(name);
  };
  const headers = { "X-Workspace-Code": "TRON_CORE", "Accept-Language": "en-US" };
  const api = async (method, path, data) => {
    const response = await page.request.fetch(`${apiBase}${path}`, { method, headers, ...(data === undefined ? {} : { data }) });
    const payload = await response.json();
    if (!response.ok()) throw new Error(`${path}: ${JSON.stringify(payload)}`);
    return payload.data;
  };
  const sent = [];
  page.on("request", request => {
    if (request.method() === "POST" && /(?:messages\/stream|steer-message)$/.test(new URL(request.url()).pathname)) {
      sent.push({ path: new URL(request.url()).pathname, input: request.postDataJSON() });
    }
  });
  await page.getByRole("button", { name: "New Chat", exact: true }).click();
  await page.getByRole("button", { name: "Select chat model", exact: true }).click();
  await page.getByRole("menuitemradio", { name: modelName, exact: true }).click();
  const editor = page.locator('[data-theme-part="chat-input"]');
  await editor.fill("/");
  await page.getByRole("listbox", { name: "Commands", exact: true }).waitFor();
  check("slash offers goal only", await page.getByRole("option").allTextContents(), ["/goalSet the execution objective for this chat"]);
  await editor.fill("$");
  await page.getByRole("listbox", { name: "Skills", exact: true }).waitFor();
  check("dollar offers effective skill", (await page.getByRole("option").allTextContents()).some(text => text.includes(skillName)));
  await page.getByRole("option").first().click();
  check("select inserts stable skill identity", await editor.inputValue(), `$${skillName} `);
  const send = async content => {
    const received = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("messages/stream"));
    await editor.fill(content); await editor.press("Enter");
    const response = await received;
    const body = await response.text();
    check(`send accepted: ${content}`, response.status(), 200);
    check(`send finished: ${content}`, body.includes("RUN_FINISHED"));
    check(`no run error: ${content}`, body.includes('event: RUN_ERROR'), false);
    return body;
  };
  const first = await send(`$${skillName} ordinary-acceptance`);
  check("selected SKILL.md reached the model", first.includes("skill-loaded"));
  check("ordinary send carries requestedSkills", sent.at(-1).input.requestedSkills, [skillName]);
  const session = new URL(page.url()).searchParams.get("sessionCode");
  if (!session) throw new Error("persisted session identity missing");
  let heldResponse;
  const hold = async suffix => {
    const admitted = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("messages/stream"));
    await editor.fill(`hold-run ${suffix}`); await editor.press("Enter");
    heldResponse = await admitted;
    await page.getByRole("button", { name: "Stop", exact: true }).waitFor();
    const deadline = Date.now() + 15000;
    while (Date.now() < deadline) {
      const status = await (await page.request.get(`${modelBase}/status`)).json();
      if (status.pending > 0) return;
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    throw new Error("fixture model did not reach its hold boundary");
  };
  const release = async () => { await page.request.get(`${modelBase}/release`); };
  const idle = async () => {
    const deadline = Date.now() + 20000;
    let settled = 0;
    while (Date.now() < deadline) {
      const run = await api("GET", `/api/v1/chats/sessions/get-active-run?sessionCode=${session}`);
      const quiet = run.activeRun === null
        && !await page.getByRole("button", { name: "Stop", exact: true }).isVisible()
        && !await page.getByRole("region", { name: "Queued", exact: true }).isVisible();
      settled = quiet ? settled + 1 : 0;
      if (settled >= 2) return;
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    throw new Error("session did not become idle");
  };
  const steer = async content => {
    const ack = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("steer-message"));
    await editor.fill(content); await editor.press("Meta+Enter");
    const response = await ack;
    check(`steer accepted: ${content}`, response.status(), 200);
    return (await response.json()).data;
  };
  await hold("queue-and-steer");
  await editor.fill(`$${skillName} queued-acceptance`); await editor.press("Enter");
  await page.getByRole("region", { name: "Queued", exact: true }).waitFor();
  await page.getByRole("button", { name: "Edit queued message", exact: true }).click();
  check("queue editing restores skill token", await editor.inputValue(), `$${skillName} queued-acceptance`);
  await editor.fill(`$${skillName} edited-queue-acceptance`); await editor.press("Enter");
  const ack = await steer(`$${skillName} immediate-acceptance`);
  check("steer persists requestedSkills", ack.message.requestedSkills, [skillName]);
  const queueFinished = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("messages/stream") && response.request().postDataJSON()?.content.includes("edited-queue-acceptance"));
  await release(); check("held run completes without error", (await heldResponse.text()).includes("event: RUN_ERROR"), false);
  const queuedBody = await (await queueFinished).text();
  check("queued run finishes", queuedBody.includes('event: RUN_ERROR'), false);
  await idle();
  const queuedRequest = sent.find(item => item.input.content.includes("edited-queue-acceptance"));
  check("queued send retains explicit selection", queuedRequest?.input.requestedSkills, [skillName]);
  const messages = await api("GET", `/api/v1/chats/sessions/${session}/messages`);
  check("product history persists explicit choices", messages.some(message => message.content.includes("edited-queue-acceptance") && message.requestedSkills.includes(skillName)));
  const history = await api("POST", `/api/v1/chats/sessions/${session}/history/query`, { size: 100, cursor: null });
  check("native history persists explicit choices", history.records.messages.some(message => message.requestedSkills.includes(skillName)));
  await page.reload();
  await page.getByRole("region", { name: "Chat transcript", exact: true }).getByText(`$${skillName} edited-queue-acceptance`, { exact: true }).waitFor({ timeout: 15000 });
  checks.push("history refresh restores selected message");
  await send("/goal ordinary-goal-acceptance");
  const latestGoal = async () => (await api("GET", `/api/v1/chats/sessions/get-latest-goal?sessionCode=${session}`))?.goal;
  check("ordinary goal command persists objective", (await latestGoal()).objectiveMarkdown, "ordinary-goal-acceptance");
  await hold("queued-goal-setup");
  await editor.fill("/goal queued-goal-acceptance"); await editor.press("Enter");
  check("queued goal has no early side effect", (await latestGoal()).objectiveMarkdown, "ordinary-goal-acceptance");
  const goalQueued = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("messages/stream") && response.request().postDataJSON()?.content === "/goal queued-goal-acceptance");
  await release(); check("held run completes without error", (await heldResponse.text()).includes("event: RUN_ERROR"), false); check("queued goal completes without error", (await (await goalQueued).text()).includes("event: RUN_ERROR"), false); await idle();
  check("queued goal executes when admitted", (await latestGoal()).objectiveMarkdown, "queued-goal-acceptance");
  await hold("immediate-goal-setup");
  await steer("/goal immediate-goal-acceptance");
  check("immediate goal command persists objective", (await latestGoal()).objectiveMarkdown, "immediate-goal-acceptance");
  await release(); check("held run completes without error", (await heldResponse.text()).includes("event: RUN_ERROR"), false); await idle();
  await hold("revocation-setup");
  await editor.fill(`$${skillName} revoked-queue-acceptance`); await editor.press("Enter");
  await api("PUT", "/api/v1/agents/registry/internal/main-agent/skills", { skillNames: [] });
  await page.evaluate(() => window.dispatchEvent(new Event("tron:capability-resources-changed")));
  const rejected = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("messages/stream") && response.status() === 403);
  await release(); await rejected;
  check("rejected queue retains explicit intent", sent.at(-1).input.requestedSkills, [skillName]);
  await page.getByRole("button", { name: "Edit queued message", exact: true }).waitFor();
  check("rejected queue remains editable", await page.getByRole("button", { name: "Edit queued message", exact: true }).isVisible());
  const after = await api("GET", `/api/v1/chats/sessions/${session}/messages`);
  check("revoked queue is never persisted as accepted", after.some(message => message.content.includes("revoked-queue-acceptance")), false);
  await page.getByRole("button", { name: "Remove queued message", exact: true }).click();
  const historical = page.locator(".ef-chat__message-entry").filter({ has: page.getByText(`$${skillName} ordinary-acceptance`, { exact: true }) });
  await historical.getByRole("button", { name: "Edit message", exact: true }).click();
  check("history editing restores saved token", await editor.inputValue(), `$${skillName} ordinary-acceptance`);
  const historyRejected = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("messages/stream") && response.status() === 403);
  await editor.press("Enter"); await historyRejected;
  check("history editing preserves selection after revocation", sent.at(-1).input.requestedSkills, [skillName]);
  await editor.fill("");
  const forbidden = await page.request.post(`${apiBase}/api/v1/chats/sessions/steer-message`, { headers, data: { sessionCode: session, expectedRunCode: "forged-run", content: "forged selection", attachments: [], requestedSkills: [skillName] } });
  check("forged steer selection is forbidden", forbidden.status(), 403);
  await api("PUT", "/api/v1/agents/registry/internal/main-agent/skills", { skillNames: [skillName] });
  await page.evaluate(() => window.dispatchEvent(new Event("tron:capability-resources-changed")));
  await editor.fill("$");
  await page.getByRole("option").first().waitFor();
  await page.screenshot({ path: "output/playwright/tron-composer-integration.png", animations: "disabled" });
  return { session, passed: checks.length, checks };
}
