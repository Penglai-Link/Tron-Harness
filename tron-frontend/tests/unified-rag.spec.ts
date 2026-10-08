import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

function previewPdf() {
  const stream = "BT /F1 20 Tf 40 220 Td (RAG original preview) Tj ET";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 300] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    `<< /Length ${Buffer.byteLength(stream)} >>\nstream\n${stream}\nendstream`,
  ];
  let pdf = "%PDF-1.4\n";
  const offsets = [0];
  objects.forEach((object, index) => {
    offsets.push(Buffer.byteLength(pdf));
    pdf += `${index + 1} 0 obj\n${object}\nendobj\n`;
  });
  const xref = Buffer.byteLength(pdf);
  pdf += `xref\n0 ${offsets.length}\n0000000000 65535 f \n`;
  pdf += offsets.slice(1).map(offset => `${String(offset).padStart(10, "0")} 00000 n \n`).join("");
  pdf += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF`;
  return Buffer.from(pdf);
}

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/settings/rag", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });
for (const locale of ["zh-CN", "en-US"] as const) {
  test(`Fixed ingestion and authorized retrieval in ${locale}`, async ({ page }, info) => {
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    const zh = locale === "zh-CN";
    const settings = { fulltext: true, vector: true, graph: false, embeddingModelCode: null, embeddingDimension: 1024, graphModelCode: null, rerankModelCode: null };
    const counts = { sourceCount: 1, rebuildSourceCount: 0, chunkCount: 12, vectorCount: 12, graphCount: 0 };
    const corpora = [
      { id: "memory-card", name: "Memory", kind: "builtin", knowledgeBaseId: null, settings, counts: { ...counts, sourceCount: 0, chunkCount: 0, vectorCount: 0 } },
      { id: "skill", name: "Skills", kind: "builtin", knowledgeBaseId: null, settings, counts: { ...counts, rebuildSourceCount: 1 } },
      { id: "corpus-evidence", name: "Evidence Library", kind: "knowledgeBase", knowledgeBaseId: "kb-evidence", settings, counts: { ...counts, rebuildSourceCount: 1 } },
    ];
    let taskState = "RUNNING";
    const actions: string[] = [];
    const rebuildRequests: unknown[] = [];
    let rejectRebuild = true;
    let rejectDocument = true;
    let rejectSearch = true;
    let rejectTaskList = false;
    let rejectedTaskLists = 0;
    const notification = (message: string) => page.locator('[data-slot="toast-viewport"]').getByRole("dialog").filter({ hasText: message });
    const documentRequests: string[] = [];
    const resourceRequests: string[] = [];
    await page.route("**/api/v1/knowledge-bases/**", async route => {
      const path = new URL(route.request().url()).pathname;
      expect(route.request().method()).toBe("GET");
      documentRequests.push(path);
      if (path === "/api/v1/knowledge-bases/kb-evidence" && rejectDocument) {
        return route.fulfill({ status: 403, json: { code: "FORBIDDEN", message: "Document access denied", data: null } });
      }
      if (path === "/api/v1/knowledge-bases/kb-evidence/resources/original-one/binary") {
        resourceRequests.push(path);
        return route.fulfill({ contentType: "application/pdf", body: previewPdf() });
      }
      const data = path === "/api/v1/knowledge-bases/kb-evidence/documents/doc-one/chunks"
        ? { items: [{ textUnitCode: "chunk-one", ordinal: 0, text: "orchid published chunk evidence", headingPath: [], metadata: { location: { page: 2, paragraph: 3, charStart: 0, charEnd: 31 } } }], total: 1, page: 1, pageSize: 15 }
        : { id: "kb-evidence", name: "Evidence Library", description: "", documentCount: 1, agentCodes: [], defaultChunkConfig: { mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2 }, documents: [{ id: "doc-one", name: "Guideline", fileName: "Guideline.pdf", resourceCode: "original-one", resourceRevision: 1, chunkConfig: null, status: "READY", error: null, chunkCount: 12, indexRevision: 1 }] };
      expect(["/api/v1/knowledge-bases/kb-evidence", "/api/v1/knowledge-bases/kb-evidence/documents/doc-one/chunks"]).toContain(path);
      return route.fulfill({ json: { code: "0", message: "ok", data } });
    });
    await page.route("**/api/v1/rag/**", async route => {
      const path = new URL(route.request().url()).pathname;
      let data: unknown;
      if (path.endsWith("/settings")) {
        const value = route.request().postDataJSON();
        expect(value).toMatchObject({ embeddingDimension: 1024, fulltext: true, vector: true, graph: false, graphModelCode: "PERSON_AI:ROUTE:ChatAuto" });
        actions.push("configure"); data = value;
      } else if (path.endsWith("/corpora")) data = corpora;
      else if (path.endsWith("/cancel")) { taskState = "CANCELLED"; actions.push("cancel"); data = {}; }
      else if (path.endsWith("/retry")) { taskState = "RUNNING"; actions.push("retry"); data = {}; }
      else if (path.endsWith("/rebuild")) { rebuildRequests.push({ taskId: path.split("/").at(-2) }); data = { accepted: true }; }
      else if (path.endsWith("/tasks") && route.request().method() === "POST") {
        const value = route.request().postDataJSON();
        expect(value.documentId).toBeTruthy();
        rebuildRequests.push(value);
        if (value.documentId === "doc-failed" && rejectRebuild) return route.fulfill({ status: 400, json: { code: "REBUILD_FAILED", message: "Synthetic rebuild failure", data: null } });
        data = { accepted: true };
      }
      else if (path.endsWith("/tasks")) {
        if (rejectTaskList) {
          rejectedTaskLists += 1;
          return route.fulfill({ status: 503, json: { code: "UNAVAILABLE", message: "Task list unavailable", data: null } });
        }
        const task = { documentId: null, corpusId: "corpus-evidence", stage: "PARSING", completedUnits: 0, totalUnits: 0, error: null, createdAt: 1, updatedAt: 2 };
        data = [
          { ...task, id: "queued-first", sourceCode: "queued-first", title: "Queued first", state: "QUEUED" },
          { ...task, id: "completed-one", documentId: "doc-completed", sourceCode: "completed-one", title: "Completed record", state: "COMPLETED" },
          { ...task, id: "failed-one", documentId: "doc-failed", sourceCode: "failed-one", title: "Failed record", state: "FAILED" },
          { ...task, id: "cancelled-one", sourceCode: "cancelled-one", title: "Cancelled record", state: "CANCELLED" },
          { ...task, id: "task-one", documentId: "doc-one", sourceCode: "source-one", title: "Guideline.pdf", state: taskState, stage: "EMBEDDING", completedUnits: 3, totalUnits: 12 },
          { ...task, id: "queued-second", sourceCode: "queued-second", title: "Queued second", state: "QUEUED" },
          { ...task, id: "memory-failed", sourceCode: "memory-failed", corpusId: "memory-card", title: "Failed memory", state: "FAILED" },
        ];
      }
      else if (path.endsWith("/recall")) {
        expect(route.request().postDataJSON()).toEqual({ query: "orchid evidence", limit: 8, corpusIds: [] });
        if (rejectSearch) return route.fulfill({ status: 404, json: { code: "NOT_FOUND", message: "API route not found", data: null } });
        data = { query: "orchid evidence", authorizedCorpusIds: ["corpus-evidence"], durationMs: 28, warnings: ["Vector retrieval unavailable: model outage"], stages: [
          { corpusId: "corpus-evidence", channel: "fulltext", status: "COMPLETED", hitCount: 1, durationMs: 4, error: null },
          { corpusId: "corpus-evidence", channel: "vector", status: "FAILED", hitCount: 0, durationMs: 20, error: "model outage" },
          { corpusId: "", channel: "fusion", status: "COMPLETED", hitCount: 1, durationMs: 1, error: null },
        ], evidence: [{ corpusId: "corpus-evidence", sourceCode: "source-one", sourceVersion: "v1", chunkCode: "chunk-one", title: "Guideline.pdf", text: "orchid original source evidence", location: { page: 2, paragraph: 3 }, source: { documentId: "doc-one" }, channels: ["fulltext"], score: 0.016 }] };
      } else throw new Error(`Unexpected RAG endpoint ${path}`);
      await route.fulfill({ json: { code: "0", message: "ok", data } });
    });
    await page.route("**/api/v1/models/platform**", route => route.fulfill({ json: { code: "0", message: "ok", data: [] } }));
    await page.route("**/api/v1/agents/workspaces/TRON_CORE/internal", route => route.fulfill({ json: { code: "0", message: "ok", data: [{ agentCode: "main-agent", name: "Main Agent" }] } }));
    await page.route("**/api/v1/agents/workspaces/TRON_EMPTY/internal", route => route.fulfill({ json: { code: "0", message: "ok", data: [] } }));
    await page.route("**/api/v1/admin/workspaces/options**", route => route.fulfill({json:{code:"0",message:"ok",data:[{workspaceCode:"TRON_CORE",name:"Tron Core",label:"Tron Core",status:"Active"},{workspaceCode:"TRON_EMPTY",name:"Empty Workspace",label:"Empty Workspace",status:"Active"}]}}));
    await page.goto(`${frontend!.baseUrl}/settings/rag`);
    await expect(page.getByRole("tab")).toHaveCount(3);
    await expect(page.getByRole("tab", { name: zh ? "定义" : "Definition", exact: true })).toBeVisible();
    await expect(page.getByText("Evidence Library", { exact: true })).toBeVisible();
    await expect(page.getByText(/Profile|DAG|JSON/)).toHaveCount(0);
    await page.getByRole("button", { name: zh ? "记忆的操作" : "Actions for Memory", exact: true }).click();
    await expect(page.getByRole("menuitem", { name: zh ? "重建索引" : "Rebuild indexes", exact: true })).toBeDisabled();
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: zh ? "查看技能的入库任务，1份资料需要重建" : "View ingestion jobs for Skills (1 to rebuild)", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/rag\?tab=ingestion&corpus=skill$/);
    await expect(page.getByRole("combobox", { name: zh ? "语料库" : "Corpus", exact: true })).toContainText("Skills");
    expect(rebuildRequests).toEqual([]);
    await page.getByRole("tab", { name: zh ? "定义" : "Definition", exact: true }).click();
    const row = page.getByRole("row").filter({ hasText: "Evidence Library" });
    const rowActions = row.getByRole("button", { name: zh ? "Evidence Library的操作" : "Actions for Evidence Library", exact: true });
    await expect(row.getByRole("button")).toHaveCount(2);
    await row.getByRole("button", { name: zh ? "查看Evidence Library的入库任务，1份资料需要重建" : "View ingestion jobs for Evidence Library (1 to rebuild)", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/rag\?tab=ingestion&corpus=corpus-evidence$/);
    await expect(page.getByRole("combobox", { name: zh ? "语料库" : "Corpus", exact: true })).toContainText("Evidence Library");
    await expect(page.getByText("Failed memory", { exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: zh ? "重建索引" : "Rebuild indexes", exact: true })).toBeDisabled();
    expect(rebuildRequests).toEqual([]);
    await page.reload();
    await expect(page.getByRole("combobox", { name: zh ? "语料库" : "Corpus", exact: true })).toContainText("Evidence Library");
    await page.getByRole("tab", { name: zh ? "定义" : "Definition", exact: true }).click();
    await rowActions.click();
    await expect(page.getByRole("menuitem")).toHaveCount(3);
    await expect(page.getByRole("menuitem", { name: zh ? "入库" : "Ingestion", exact: true })).toHaveCount(0);
    await expect(page.getByRole("menuitem", { name: zh ? "管理知识库" : "Manage knowledge base", exact: true })).toBeVisible();
    await page.screenshot({ path: info.outputPath(`unified-rag-definition-${locale}.png`), fullPage: true });
    await page.getByRole("menuitem", { name: zh ? "重建索引" : "Rebuild indexes", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/rag\?tab=ingestion&corpus=corpus-evidence$/);
    expect(rebuildRequests).toEqual([]);
    await page.getByRole("tab", { name: zh ? "定义" : "Definition", exact: true }).click();
    await rowActions.click();
    await page.getByRole("menuitem", { name: zh ? "配置索引" : "Configure indexes", exact: true }).click();
    const dialog = page.getByRole("dialog").filter({ hasNot: page.locator('[data-slot="toast-title"], [data-slot="toast-description"]') });
    await expect(dialog.getByRole("combobox", { name: zh ? "全文" : "Full text", exact: true })).toBeFocused();
    await expect(page.getByRole("tooltip")).toHaveCount(0);
    await expect(dialog.getByRole("switch")).toHaveCount(0);
    const vectorMode = dialog.getByRole("combobox", { name: zh ? "向量" : "Vector", exact: true });
    await expect(vectorMode).toContainText(zh ? "启用" : "Enabled");
    await vectorMode.click();
    await page.getByRole("option", { name: zh ? "禁用" : "Disabled", exact: true }).click();
    await expect(vectorMode).toContainText(zh ? "禁用" : "Disabled");
    await vectorMode.click();
    await page.getByRole("option", { name: zh ? "启用" : "Enabled", exact: true }).click();
    await expect(dialog.getByRole("combobox", { name: zh ? "图谱" : "Graph", exact: true })).toContainText(zh ? "禁用" : "Disabled");
    await expect(dialog.getByText(zh ? "按语义召回，需要 Embedding 模型。" : "Recall by meaning using an Embedding model.", { exact: true })).not.toBeVisible();
    await dialog.getByRole("button", { name: zh ? "图谱抽取模型说明" : "Help for Graph extraction model", exact: true }).hover();
    await expect(page.getByRole("tooltip")).toBeVisible();
    await expect(page.getByRole("tooltip")).toContainText(zh ? "从正文找出实体和关系" : "extracts entities and relations");
    await page.screenshot({ path: info.outputPath(`unified-rag-configuration-${locale}.png`), fullPage: true });
    const dimension = dialog.getByLabel(zh ? "向量维度" : "Embedding dimensions", { exact: true });
    await expect(dimension).toHaveValue("1024");
    await expect(dimension).toBeDisabled();
    await dialog.getByRole("combobox", { name: zh ? "图谱抽取模型" : "Graph extraction model", exact: true }).click();
    await page.getByRole("option", { name: zh ? "自动路由（Auto）" : "Automatic routing (Auto)", exact: true }).click();
    await dialog.getByRole("button", { name: zh ? "保存" : "Save", exact: true }).click();
    await expect.poll(() => actions).toContain("configure");
    await page.getByRole("button", { name: zh ? "查看入库任务" : "View ingestion jobs", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/rag\?tab=ingestion&corpus=corpus-evidence$/);
    const initialCorpusFilter = page.getByRole("combobox", { name: zh ? "语料库" : "Corpus", exact: true });
    await initialCorpusFilter.click();
    await page.getByRole("option", { name: zh ? "全部语料库" : "All corpora", exact: true }).click();
    await expect(page.getByRole("columnheader", { name: zh ? "分块数" : "Chunks", exact: true })).toBeVisible();
    await expect(page.getByRole("row").filter({ hasText: "Guideline.pdf" }).getByRole("cell", { name: "12", exact: true })).toBeVisible();
    await expect(page.locator("[aria-current=step]")).toHaveCount(0);
    const ingestionFlow = page.getByRole("list", { name: zh ? "统一入库流程" : "Unified ingestion flow", exact: true });
    await expect(ingestionFlow.getByRole("listitem")).toHaveCount(4);
    await expect(ingestionFlow.getByText(/^[1-4]$/)).toHaveCount(0);
    const statistics = page.getByRole("group", { name: zh ? "入库任务统计" : "Ingestion task statistics", exact: true });
    const expectStatistics = async (values: number[]) => {
      const labels = zh ? ["总任务数", "处理中", "失败"] : ["Total tasks", "In progress", "Failed"];
      for (const [index, label] of labels.entries()) await expect(statistics.getByText(label, { exact: true }).locator("..").locator("p").last()).toHaveText(String(values[index]));
    };
    await expectStatistics([7, 3, 2]);
    await expect(page.getByText(zh ? "启用的索引全部成功后发布新版本；失败或取消时，旧版本继续服务。" : "A new version is published only after all enabled indexes succeed. Failed or cancelled jobs leave the published version serving.", { exact: true })).toHaveCount(0);
    const taskNames = page.locator("table tbody tr td:nth-child(2)");
    await expect(taskNames).toHaveText(["Guideline.pdf", "Queued first", "Queued second", "Completed record", "Failed record", "Cancelled record", "Failed memory"]);
    const rebuild = page.getByRole("button", { name: zh ? "重建索引" : "Rebuild indexes", exact: true });
    await expect(rebuild).toBeDisabled();
    const toolbar = page.locator("[role=tabpanel] button").filter({ hasText: zh ? /重建索引|刷新/ : /Rebuild indexes|Refresh/ });
    await expect(toolbar).toHaveText(zh ? ["重建索引", "刷新"] : ["Rebuild indexes", "Refresh"]);
    await expect(page.getByRole("button", { name: zh ? "刷新" : "Refresh", exact: true })).toHaveCount(1);
    const selectTask = (name: string) => page.getByRole("checkbox", { name: zh ? `选择 ${name}` : `Select ${name}`, exact: true });
    await expect(selectTask("Guideline.pdf")).toBeDisabled();
    await expect(selectTask("Queued first")).toBeDisabled();
    const stateFilter = page.getByRole("combobox", { name: zh ? "状态" : "State", exact: true });
    await stateFilter.click();
    await page.getByRole("option", { name: zh ? "失败" : "Failed", exact: true }).click();
    await expect(taskNames).toHaveText(["Failed record", "Failed memory"]);
    await expectStatistics([7, 3, 2]);
    const corpusFilter = page.getByRole("combobox", { name: zh ? "语料库" : "Corpus", exact: true });
    await corpusFilter.click();
    await page.getByRole("option", { name: "Evidence Library", exact: true }).click();
    await expect(taskNames).toHaveText(["Failed record"]);
    await expectStatistics([6, 3, 1]);
    await selectTask("Failed record").check();
    await expect(rebuild).toBeEnabled();
    await corpusFilter.click();
    await page.getByRole("option", { name: "Memory", exact: true }).click();
    await expect(taskNames).toHaveText(["Failed memory"]);
    await expectStatistics([1, 0, 1]);
    await expect(rebuild).toBeDisabled();
    await stateFilter.click();
    await page.getByRole("option", { name: zh ? "完成" : "Completed", exact: true }).click();
    await expect(page.getByText(zh ? "暂无符合筛选条件的入库任务。" : "No ingestion jobs match these filters.", { exact: true })).toBeVisible();
    await corpusFilter.click();
    await page.getByRole("option", { name: zh ? "全部语料库" : "All corpora", exact: true }).click();
    await expect(taskNames).toHaveText(["Completed record"]);
    await stateFilter.click();
    await page.getByRole("option", { name: zh ? "全部状态" : "All states", exact: true }).click();
    await expect(taskNames.first()).toHaveText("Guideline.pdf");
    const selectAll = page.getByRole("checkbox", { name: zh ? "选择当前列表可重建的任务" : "Select rebuildable tasks in this list", exact: true });
    await selectAll.check();
    await expect(selectTask("Completed record")).toBeChecked();
    await expect(selectTask("Guideline.pdf")).not.toBeChecked();
    await selectAll.uncheck();
    await expect(rebuild).toBeDisabled();
    await selectTask("Completed record").check();
    await selectTask("Failed record").check();
    await selectTask("Failed memory").check();
    await rebuild.click();
    await expect(notification("Failed record: Synthetic rebuild failure")).toBeVisible();
    await expect(page.getByRole("main").getByRole("alert")).toHaveCount(0);
    expect(rebuildRequests).toEqual([{ corpusId: "corpus-evidence", documentId: "doc-completed" }, { corpusId: "corpus-evidence", documentId: "doc-failed" }, { taskId: "memory-failed" }]);
    await expect(selectTask("Completed record")).not.toBeChecked();
    await expect(selectTask("Failed memory")).not.toBeChecked();
    await expect(selectTask("Failed record")).toBeChecked();
    rejectRebuild = false;
    await rebuild.click();
    await expect(selectTask("Failed record")).not.toBeChecked();
    await expect(rebuild).toBeDisabled();
    expect(rebuildRequests).toHaveLength(4);
    rejectTaskList = true;
    await page.getByRole("button", { name: zh ? "刷新" : "Refresh", exact: true }).click();
    const pollingFailure = notification("Task list unavailable");
    await expect(pollingFailure).toBeVisible();
    await pollingFailure.hover();
    await expect.poll(() => rejectedTaskLists, { timeout: 15000 }).toBeGreaterThanOrEqual(2);
    await expect(pollingFailure).toHaveCount(1);
    await expect(page.getByRole("main").getByRole("alert")).toHaveCount(0);
    rejectTaskList = false;
    await page.getByRole("button", { name: zh ? "刷新" : "Refresh", exact: true }).click();
    await page.screenshot({ path: info.outputPath(`unified-rag-ingestion-${locale}.png`), fullPage: true });
    const taskRow = page.getByRole("row").filter({ hasText: "Guideline.pdf" });
    const taskActions = taskRow.getByRole("button", { name: zh ? "Guideline.pdf的操作" : "Actions for Guideline.pdf", exact: true });
    await expect(taskRow.getByRole("button")).toHaveCount(1);
    const ingestionUrl = page.url();
    await taskActions.click();
    await expect(page.getByRole("menuitem")).toHaveCount(3);
    await page.getByRole("menuitem", { name: zh ? "查看分块" : "View chunks", exact: true }).click();
    await expect(notification("Document access denied")).toBeVisible();
    await expect(dialog.getByRole("alert")).toHaveCount(0);
    expect(documentRequests.length).toBeGreaterThan(0);
    expect(documentRequests.every(path => path === "/api/v1/knowledge-bases/kb-evidence")).toBe(true);
    await expect(page).toHaveURL(ingestionUrl);
    rejectDocument = false;
    await dialog.getByRole("button", { name: zh ? "重试" : "Retry", exact: true }).click();
    await expect(dialog.getByText("orchid published chunk evidence", { exact: true })).toBeVisible();
    await expect(dialog).toContainText(zh ? "第 2 页" : "Page 2");
    await expect(page).toHaveURL(ingestionUrl);
    await page.screenshot({ path: info.outputPath(`unified-rag-ingestion-chunks-${locale}.png`), fullPage: true });
    await dialog.getByRole("button", { name: zh ? "关闭" : "Close", exact: true }).click();
    await taskActions.click();
    await page.getByRole("menuitem", { name: zh ? "查看原文" : "View original", exact: true }).click();
    await expect(dialog.getByRole("heading", { name: "Guideline.pdf", exact: true })).toBeVisible();
    await expect(dialog.locator("iframe")).toHaveAttribute("src", /^blob:/);
    expect(resourceRequests.length).toBeGreaterThan(0);
    expect([...new Set(resourceRequests)]).toEqual(["/api/v1/knowledge-bases/kb-evidence/resources/original-one/binary"]);
    await expect(page).toHaveURL(ingestionUrl);
    await page.screenshot({ path: info.outputPath(`unified-rag-ingestion-original-${locale}.png`), fullPage: true });
    await dialog.getByRole("button", { name: zh ? "关闭" : "Close", exact: true }).last().click();
    await taskActions.click();
    await page.getByRole("menuitem", { name: zh ? "取消" : "Cancel", exact: true }).click();
    await expect.poll(() => actions).toContain("cancel");
    await expect(taskRow).toContainText(zh ? "已取消" : "Cancelled");
    await taskActions.click();
    await page.getByRole("menuitem", { name: zh ? "重试" : "Retry", exact: true }).click();
    await expect.poll(() => actions).toContain("retry");
    await page.getByRole("tab", { name: zh ? "检索" : "Retrieval", exact: true }).click();
    await expect(page.getByRole("list", { name: zh ? "统一检索流程" : "Unified retrieval flow", exact: true }).getByRole("listitem")).toHaveCount(4);
    await expect(page.getByRole("combobox", { name: zh ? "智能体" : "Agent", exact: true })).toHaveCount(0);
    await expect(page.getByRole("combobox", { name: zh ? "空间" : "Workspace", exact: true })).toHaveCount(0);
    await page.screenshot({ path: info.outputPath(`unified-rag-retrieval-flow-${locale}.png`), fullPage: true });
    await expect(page.locator('[data-slot="toast-viewport"]').getByRole("dialog")).toHaveCount(0, { timeout: 15000 });
    await page.getByLabel(zh ? "检索问题" : "Question", { exact: true }).fill("orchid evidence");
    await page.getByRole("button", { name: zh ? "检索" : "Search", exact: true }).click();
    await expect(notification("API route not found")).toBeVisible();
    await expect(notification("API route not found")).toBeInViewport({ ratio: 1 });
    await expect(page.getByRole("button", { name: zh ? "检索" : "Search", exact: true })).toBeEnabled();
    await expect(page.getByRole("main").getByRole("alert")).toHaveCount(0);
    await page.screenshot({ path: info.outputPath(`unified-rag-error-toast-${locale}.png`), fullPage: true, animations: "disabled" });
    rejectSearch = false;
    await page.getByRole("button", { name: zh ? "检索" : "Search", exact: true }).click();
    await expect(page.getByText("orchid original source evidence", { exact: true })).toBeVisible();
    await expect(notification("Vector retrieval unavailable: model outage")).toBeVisible();
    await expect(page.getByRole("main").getByText("Vector retrieval unavailable: model outage", { exact: true })).toHaveCount(0);
    await page.getByRole("button", { name: zh ? "向量失败原因" : "Failure details for Vector", exact: true }).hover();
    await expect(page.getByRole("tooltip")).toHaveText("model outage");
    await expect(page.getByRole("button", { name: zh ? "查看分块与原文" : "View chunks and original", exact: true })).toBeVisible();
    await page.screenshot({ path: info.outputPath(`unified-rag-${locale}.png`), fullPage: true });
  });
}
