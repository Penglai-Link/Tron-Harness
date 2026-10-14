/** CLI acceptance with the isolated three-stage fixture model. Each stage sends
 * public progress text and then holds its response until /timing/release.
 */
export async function verifyAguiTiming(page, { apiBase, modelBase }) {
  const checks = [];
  const check = (name, value) => { if (!value) throw new Error(name); checks.push(name); };
  const headers = { "X-Workspace-Code": "TRON_CORE", "Accept-Language": "en-US" };
  const api = async (path, data) => {
    const response = await page.request.fetch(`${apiBase}${path}`, { headers, ...(data ? { method: "POST", data } : {}) });
    const payload = await response.json();
    if (!response.ok()) throw new Error(JSON.stringify(payload));
    return payload.data;
  };
  await page.getByRole("button", { name: "New Chat", exact: true }).click();
  await page.getByRole("button", { name: "Select chat model", exact: true }).click();
  await page.getByRole("menuitemradio", { name: "Composer Preview", exact: true }).click();
  const marker = `AGUI_TIMING_${Date.now()}`;
  const editor = page.locator('[data-theme-part="chat-input"]');
  const streamed = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("messages/stream"));
  await editor.fill(`${marker} 请逐步检查目录结构和模块边界，再给出结论。`);
  await editor.press("Enter");
  const response = await streamed;
  check("SSE admitted", response.status() === 200);
  const summaries = ["正在检查目录结构。", "正在检查模块边界。", "最终答复：结构检查完成。"];
  const ids = [];
  let session;
  for (let stage = 0; stage < summaries.length; stage++) {
    const deadline = Date.now() + 20000;
    let held = false;
    while (Date.now() < deadline) {
      const status = await (await page.request.get(`${modelBase}/timing/status`)).json();
      if (status.pending.some(item => item.marker === marker && item.stage === stage)) { held = true; break; }
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    check(`model stage ${stage + 1} is still generating`, held);
    const card = page.locator('article[data-theme-part="chat-message-assistant"]').filter({ hasText: summaries[stage] });
    await card.waitFor({ timeout: 15000 });
    const id = await card.getAttribute("data-message-id");
    check(`stage ${stage + 1} has an independent protocol ID`, Boolean(id) && !ids.includes(id) && !id.startsWith("LOCAL-"));
    ids.push(id);
    check(`stage ${stage + 1} is visible before completion`, await page.getByRole("button", { name: "Stop", exact: true }).isVisible());
    check(`stage ${stage + 1} is streaming on its own card`, await card.getAttribute("data-streaming") === "true");
    check(`stage ${stage + 1} card count`, await page.locator('article[data-theme-part="chat-message-assistant"]').count() === stage + 1);
    if (stage === 0) await card.evaluate(node => { window.__firstAguiCard = node; });
    if (stage > 0) {
      const first = page.locator(`[data-message-id="${ids[0]}"]`);
      check(`stage ${stage + 1} preserves the first card`, await first.evaluate(node => node === window.__firstAguiCard));
      check(`stage ${stage + 1} does not append into an earlier card`, (await first.innerText()).includes(summaries[0]) && !(await first.innerText()).includes(summaries[stage]));
      check(`stage ${stage + 1} keeps the first card settled`, await first.getAttribute("data-streaming") === null);
    }
    session = new URL(page.url()).searchParams.get("sessionCode");
    const protocol = await api(`/api/v1/agents/agui/streams/${session}/protocol-events`);
    check(`stage ${stage + 1} delta is already durably recorded`, protocol.some(event => event.data.messageId === id && event.data.type === "TEXT_MESSAGE_CONTENT"));
    await page.screenshot({ path: `output/playwright/agui-timing-stage-${stage + 1}.png`, animations: "disabled" });
    await page.request.get(`${modelBase}/timing/release`);
  }
  const body = await response.text();
  check("run has no protocol or execution error", !body.includes("event: RUN_ERROR"));
  const frames = body.split("\n").filter(line => line.startsWith("data: ")).flatMap(line => { try { return [JSON.parse(line.slice(6))]; } catch { return []; } });
  let previousEnd = -1;
  for (const id of ids) {
    const start = frames.findIndex(frame => frame.type === "TEXT_MESSAGE_START" && frame.messageId === id);
    const delta = frames.findIndex(frame => frame.type === "TEXT_MESSAGE_CONTENT" && frame.messageId === id);
    const end = frames.findIndex(frame => frame.type === "TEXT_MESSAGE_END" && frame.messageId === id);
    check(`message ${id} start/content/end follows the previous step`, start > previousEnd && delta > start && end > delta);
    previousEnd = end;
  }
  check("all step messages end before run completion", frames.findIndex(frame => frame.type === "RUN_FINISHED") > previousEnd);
  await page.getByRole("button", { name: "Stop", exact: true }).waitFor({ state: "hidden" });
  const history = await api(`/api/v1/chats/sessions/${session}/history/query`, { size: 100, cursor: null });
  check("history uses the same three message IDs", ids.every(id => history.records.messages.some(message => message.id === id)));
  await page.reload();
  for (const [index, id] of ids.entries()) {
    await page.locator(`[data-message-id="${id}"]`).waitFor();
    check(`history restores step ${index + 1} once`, await page.getByText(summaries[index], { exact: true }).count() === 1);
  }
  check("history still has exactly three assistant cards", await page.locator('article[data-theme-part="chat-message-assistant"]').count() === 3);
  await page.screenshot({ path: "output/playwright/agui-timing-history.png", animations: "disabled" });
  return { session, passed: checks.length, ids, checks };
}
