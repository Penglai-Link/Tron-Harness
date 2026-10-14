/** Real Framework/Tron execution with a gated three-step fixture provider. */
export async function verifyTronWorkTimeline(page, { modelBase, apiBase }) {
  const checks = [], check = (name, value) => { if (!value) throw new Error(name); checks.push(name); };
  await page.getByRole("button", { name: "New Chat", exact: true }).click();
  await page.getByRole("button", { name: "Select chat model", exact: true }).click();
  await page.getByRole("menuitemradio", { name: "Composer Preview", exact: true }).click();
  const marker = `AGUI_TIMELINE_${Date.now()}`;
  const streamed = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("messages/stream"));
  await page.locator('[data-theme-part="chat-input"]').fill(`${marker} 请分步骤检查目录与模块边界，再给出最终答复。`);
  await page.locator('[data-theme-part="chat-input"]').press("Enter");
  const response = await streamed;
  check("real SSE admitted", response.status() === 200);
  const work = page.getByRole("region", { name: "Agent work", exact: true });
  const thoughts = ["先检查目录结构。", "继续检查模块边界。", "资料已经齐全，整理最终回答。"];
  const outputs = ["正在检查目录结构。", "正在检查模块边界。", "最终答复：结构检查完成。"];
  const ids = [];
  for (let stage = 0; stage < 3; stage++) {
    await work.getByText(thoughts[stage], { exact: true }).waitFor({ timeout: 25000 });
    const card = work.locator('article[data-theme-part="chat-message-assistant"]').filter({ hasText: outputs[stage] });
    await card.waitFor(); ids.push(await card.getAttribute("data-message-id"));
    check(`phase ${stage + 1} is visible while execution is held`, await page.getByRole("button", { name: "Stop", exact: true }).isVisible());
    check(`phase ${stage + 1} requires no process expansion`, await work.locator('.ef-chat__rail-summary[aria-expanded]').count() === 0);
    const status = work.locator('.ef-chat__rail-current--working');
    check(`phase ${stage + 1} has a fixed final working row`, await status.textContent() === "Working" && await status.evaluate(node => node === node.parentElement.lastElementChild));
    const sequence = await work.locator('[data-work-item]').evaluateAll(nodes => nodes.map(node => node.dataset.workItem));
    const expected = Array.from({ length: stage + 1 }, (_, index) => [...(index ? ["tool"] : []), "reasoning", "message"]).flat();
    check(`phase ${stage + 1} follows native event order`, JSON.stringify(sequence) === JSON.stringify(expected));
    await page.screenshot({ path: `output/playwright/tron-work-timeline-phase-${stage + 1}.png`, animations: "disabled" });
    await page.request.get(`${modelBase}/timing/release`);
  }
  const frames = await response.text();
  check("execution completes successfully", !frames.includes("event: RUN_ERROR") && frames.includes("RUN_FINISHED"));
  await page.getByRole("button", { name: "Stop", exact: true }).waitFor({ state: "hidden" });
  const summary = work.getByRole("button", { name: /Worked for/ }); await summary.waitFor();
  check("all process cards fold together", await summary.getAttribute("aria-expanded") === "false" && await work.locator('[data-work-item]').count() === 0);
  check("only the final assistant card remains outside the process", await page.locator('article[data-theme-part="chat-message-assistant"]').count() === 1 && await page.locator(`[data-message-id="${ids[2]}"]`).isVisible());
  check("working row is hidden after completion", await work.locator('.ef-chat__rail-current--working').count() === 0);
  const session = new URL(page.url()).searchParams.get("sessionCode");
  const historyResponse = await page.request.post(`${apiBase}/api/v1/chats/sessions/${session}/history/query`, { headers: { "X-Workspace-Code": "TRON_CORE", "Accept-Language": "en-US" }, data: { size: 100, cursor: null } });
  const history = (await historyResponse.json()).data;
  check("durable history has the same message identities", ids.every(id => history.records.messages.some(message => message.id === id)));
  await summary.click();
  await work.getByText(outputs[0], { exact: true }).waitFor();
  check("expanded process retains both actual tool calls", await work.locator('[data-work-item="tool"]').count() === 2);
  check("final answer is not duplicated on expansion", await page.getByText(outputs[2], { exact: true }).count() === 1);
  await page.reload(); await summary.waitFor();
  check("history restores folded process and separate final", await summary.getAttribute("aria-expanded") === "false" && await page.locator('article[data-theme-part="chat-message-assistant"]').count() === 1);
  await summary.click();
  await work.getByText(thoughts[0], { exact: true }).waitFor();
  check("history preserves all three public thoughts", await work.locator('[data-work-item="reasoning"]').count() === 3);
  check("history restores both intermediate outputs and both tools", await work.locator('[data-work-item="message"]').count() === 2 && await work.locator('[data-work-item="tool"]').count() === 2);
  const restoredOrder = await work.locator('[data-work-item]').evaluateAll(nodes => nodes.map(node => node.dataset.workItem));
  check("history preserves the native process order", JSON.stringify(restoredOrder) === JSON.stringify(["reasoning", "message", "tool", "reasoning", "message", "tool", "reasoning"]));
  await summary.click();
  await page.screenshot({ path: "output/playwright/tron-work-timeline-final.png", animations: "disabled" });
  return { session, passed: checks.length, ids, checks };
}
