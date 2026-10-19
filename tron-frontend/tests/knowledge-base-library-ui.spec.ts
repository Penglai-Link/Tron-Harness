import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/knowledge-base", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const locale of ["en-US", "zh-CN"] as const) {
  test(`Catalog counts, row navigation and guarded deletion in ${locale}`, async ({ page }, testInfo) => {
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    await page.route("**/api/v1/chats/session-groups?*", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: [] } }));
    let items = [
      { id: "kb-full", name: "Full library", description: "Library description", documentCount: 1, authorizedAgentCount: 2 },
      { id: "kb-empty", name: "Empty library", description: "Empty description", documentCount: 0, authorizedAgentCount: 0 },
    ];
    const deleted: string[] = [];
    const grantCodes: Record<string,string[]> = {"kb-full":["main-agent","expert-agent"],"kb-empty":[]};
    await page.route("**/api/v1/dudes", route => route.fulfill({ json: { code: "0", message: "ok", data: [] } }));
    await page.route("**/api/v1/agents/registry/internal", route => route.fulfill({json:{code:"0",message:"ok",data:[{agentCode:"main-agent",name:"Main Agent"},{agentCode:"expert-agent",name:"Expert Agent"}]}}));
    await page.route("**/api/v1/knowledge-bases**", async route => {
      const url = new URL(route.request().url());
      if (url.pathname.endsWith("/agent-grants")) {
        expect(url.searchParams.has("workspaceCode")).toBe(false);
        const id = url.pathname.split("/").at(-2)!;
        if (route.request().method() === "PUT") {
          expect(route.request().postDataJSON()).toEqual({agentCodes:["main-agent"]});
          grantCodes[id] = ["main-agent"];
          items = items.map(item => item.id === id ? {...item, authorizedAgentCount:1} : item);
          return route.fulfill({json:{code:"0",message:"ok",data:{}}});
        }
        return route.fulfill({json:{code:"0",message:"ok",data:{agentCodes:grantCodes[id]}}});
      }
      if (url.pathname.endsWith("/platform")) {
        expect(route.request().headers()["accept-language"]).toBe(locale);
        return route.fulfill({json:{code:"0",message:"ok",data:items}});
      }
      expect(url.searchParams.has("workspaceCode")).toBe(false);
      let data: unknown = items;
      if (route.request().method() === "DELETE") {
        expect(url.pathname).toBe("/api/v1/knowledge-bases/kb-empty");
        expect(route.request().headers()["accept-language"]).toBe(locale);
        deleted.push(url.pathname); items = items.filter(item => item.id !== "kb-empty"); data = {};
      } else if (url.pathname.endsWith("/kb-full")) {
        data = { ...items[0], documents: [], agentCodes: ["a", "b"], defaultChunkConfig: { mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2 } };
      }
      await route.fulfill({ json: { code: "0", message: "ok", data } });
    });
    await page.goto(`${frontend!.baseUrl}/settings/knowledge-base`);
    await expect(page.locator("main details")).toHaveCount(0);
    await expect(page.getByRole("button", { name: locale === "en-US" ? "RAG administration" : "RAG 管理", exact: true })).toHaveCount(0);
    const full = page.getByRole("row").filter({ hasText: "Full library" });
    const empty = page.getByRole("row").filter({ hasText: "Empty library" });
    await expect(page.getByRole("columnheader",{name:locale === "en-US" ? "Authorized agent count" : "授权智能体数",exact:true})).toBeVisible();
    await expect(page.getByRole("columnheader",{name:locale === "en-US" ? "Assigned workspaces" : "已分配空间",exact:true})).toHaveCount(0);
    await expect(page.getByRole("combobox")).toHaveCount(0);
    await expect(full.locator("td").nth(3)).toHaveText("2");
    await expect(empty.locator("td").nth(3)).toHaveText("0");
    const assignName = locale === "en-US" ? "Authorize agents" : "授权智能体";
    await empty.getByRole("button", {name:/Empty library/}).click();
    await page.getByRole("menuitem", {name:assignName,exact:true}).click();
    await expect(page.getByRole("switch",{name:"Main Agent",exact:true})).not.toBeChecked();
    await expect(page.getByRole("dialog", {name:assignName,exact:true}).getByRole("combobox")).toHaveCount(0);
    await page.getByRole("switch", {name:"Main Agent",exact:true}).check();
    await expect(page.getByRole("switch", {name:"Main Agent",exact:true})).toBeChecked();
    await page.getByRole("dialog", {name:assignName,exact:true}).getByRole("button", {name:locale === "en-US" ? "Save" : "保存", exact:true}).click();
    await expect.poll(() => grantCodes["kb-empty"]).toEqual(["main-agent"]);
    await expect(page.getByRole("dialog",{name:assignName,exact:true})).toHaveCount(0);
    await expect(empty.locator("td").nth(3)).toHaveText("1");
    await full.getByRole("button", {name: /Full library/}).click();
    await expect(page).toHaveURL(/\/settings\/knowledge-base$/);
    const remove = page.getByRole("menuitem", { name: locale === "en-US" ? "Delete" : "删除", exact: true });
    await expect(remove).toHaveAttribute("aria-disabled", "true");
    await remove.locator("..").hover();
    await expect(page.getByRole("tooltip")).toContainText(locale === "en-US" ? "Delete all resources" : "请先删除知识库内的所有资源");
    await page.keyboard.press("Escape");
    await empty.getByRole("button", {name:/Empty library/}).click(); await page.getByRole("menu", {name:/Empty library/}).getByRole("menuitem", {name:locale === "en-US" ? "Delete" : "删除", exact:true}).click();
    const dialog = page.getByRole("dialog",{name:locale === "en-US" ? "Delete knowledge base?" : "删除知识库？",exact:true});
    await expect(dialog).toContainText("Empty library");
    expect(deleted).toHaveLength(0);
    await dialog.getByRole("button", { name: locale === "en-US" ? "Cancel" : "取消", exact: true }).click();
    expect(deleted).toHaveLength(0);
    await empty.getByRole("button", {name:/Empty library/}).click(); await page.getByRole("menu", {name:/Empty library/}).getByRole("menuitem", {name:locale === "en-US" ? "Delete" : "删除", exact:true}).click();
    await dialog.getByRole("button", { name: locale === "en-US" ? "Delete" : "删除", exact: true }).click();
    await expect(empty).toHaveCount(0); expect(deleted).toHaveLength(1);
    await page.screenshot({ animations: "disabled", path: testInfo.outputPath(`knowledge-catalog-${locale}.png`) });
    await full.locator("td").nth(1).click();
    await expect(page).toHaveURL(/\/settings\/knowledge-base\/kb-full$/);
    await page.getByRole("button", {name:assignName,exact:true}).click();
    await expect(page.getByRole("dialog", {name:assignName,exact:true}).getByRole("switch",{name:"Main Agent"})).toBeChecked();
    await expect(page.getByRole("dialog", {name:assignName,exact:true}).getByRole("switch",{name:"Expert Agent"})).toBeChecked();
    await page.getByRole("dialog", {name:assignName,exact:true}).getByRole("button",{name:locale === "en-US" ? "Cancel" : "取消",exact:true}).and(page.locator("[data-slot=button]")).click();
    await page.goto(`${frontend!.baseUrl}/settings/knowledge-base`);
    await full.focus(); await full.press("Enter");
    await expect(page).toHaveURL(/\/settings\/knowledge-base\/kb-full$/);
  });
}

for (const locale of ["zh-CN", "en-US"] as const) {
  test(`Create knowledge base with index settings in ${locale}`, async ({ page }, info) => {
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    const zh = locale === "zh-CN";
    const created: { name: string; description: string; indexSettings: Record<string, unknown> }[] = [];
    let failCreate = true;
    await page.route("**/api/v1/admin/platform/models**", route => route.fulfill({ json: { code: "0", message: "ok", data: [{ apiKeyRef: "", parameters: {}, baseUrl: "", modelCode: "vision-model", displayName: "Vision model", enabled: true, modelType: "TEXT", modelCapabilities: ["VISION_UNDERSTANDING"] }] } }));
    await page.route("**/api/v1/model-providers/PersonAi/catalog", route => route.fulfill({ json: { code: "0", message: "ok", data: [{ catalogModelType: "Embedding", status: "Available", modelId: "embedding-1", modelCode: "bge-m3", modelName: "BGE-M3" }] } }));
    await page.route("**/api/v1/knowledge-bases**", async route => {
      const request = route.request(); const path = new URL(request.url()).pathname;
      let data: unknown;
      if (request.method() === "POST") {
        expect(path).toBe("/api/v1/knowledge-bases");
        expect(request.headers()["accept-language"]).toBe(locale);
        created.push(request.postDataJSON());
        if (failCreate) return route.fulfill({ status: 400, json: { code: "101027005", message: "Synthetic create failure", data: {} } });
        data = { id: "kb-configured", name: "Configured", description: "Selected before upload", documentCount: 0 };
      } else if (path.endsWith("/platform")) data = [];
      else if (path.endsWith("/parsing-capabilities")) data = { defaultTier: "standard", defaultOcrMode: "Auto", availableTiers: ["standard"], parserVersion: "test", officeTier: "flash" };
      else data = { id: "kb-configured", name: "Configured", description: "Selected before upload", documentCount: 0, documents: [], agentCodes: [], defaultChunkConfig: { mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2 } };
      return route.fulfill({ json: { code: "0", message: "ok", data } });
    });
    await page.route("**/api/v1/rag/**", async route => {
      expect(route.request().method()).toBe("GET"); // Creation does not require a second configuration call.
      const data = new URL(route.request().url()).pathname.endsWith("/corpora") ? [{ id: "kb-configured", name: "Configured", kind: "knowledgeBase", knowledgeBaseId: "kb-configured", settings: created.at(-1)!.indexSettings, counts: { sourceCount: 0, rebuildSourceCount: 0, chunkCount: 0, vectorCount: 0, graphCount: 0 } }] : [];
      return route.fulfill({ json: { code: "0", message: "ok", data } });
    });
    await page.goto(`${frontend!.baseUrl}/settings/knowledge-base`);
    await page.getByRole("button", { name: zh ? "创建" : "Create", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: zh ? "创建" : "Create", exact: true });
    const name = dialog.getByLabel(zh ? "名称" : "Name", { exact: true });
    await expect(name).toBeFocused(); await name.fill("Configured");
    await dialog.getByLabel(zh ? "说明" : "Description", { exact: true }).fill("Selected before upload");
    const choose = async (field: string, option: string) => {
      await dialog.getByRole("combobox", { name: field, exact: true }).click();
      await page.getByRole("option", { name: option, exact: true }).click();
    };
    for (const [field, selected] of zh ? [["Embedding 模型", "向量自动路由"], ["图谱抽取模型", "对话自动路由"], ["重排模型", "重排自动路由"]] : [["Embedding model", "Embedding Automatic Routing"], ["Graph extraction model", "Conversation Automatic Routing"], ["Reranking model", "Rerank Automatic Routing"]]) {
      await expect(dialog.getByRole("combobox", { name: field, exact: true })).toContainText(selected);
    }
    const submit = dialog.getByRole("button", { name: zh ? "创建" : "Create", exact: true });
    await choose(zh ? "Embedding 模型" : "Embedding model", zh ? "选择模型" : "Select model");
    await expect(submit).toBeDisabled();
    await choose(zh ? "Embedding 模型" : "Embedding model", "BGE-M3 · Person-AI");
    await choose(zh ? "图谱" : "Graph", zh ? "启用" : "Enabled");
    await choose(zh ? "图谱抽取模型" : "Graph extraction model", zh ? "选择模型" : "Select model");
    await expect(submit).toBeDisabled();
    await choose(zh ? "图谱抽取模型" : "Graph extraction model", zh ? "对话自动路由" : "Conversation Automatic Routing");
    await choose(zh ? "重排模型" : "Reranking model", zh ? "关闭重排" : "Reranking off");
    await choose(zh ? "图片检索方式" : "Image retrieval", "Vision model");
    await expect(submit).toBeEnabled();
    await page.screenshot({ path: info.outputPath(`knowledge-create-index-settings-${locale}.png`), fullPage: true });
    await submit.click();
    await expect(dialog).toBeVisible(); await expect(name).toHaveValue("Configured");
    await expect(page.getByText("Synthetic create failure", { exact: true })).toBeVisible();
    await expect(submit).toBeEnabled(); failCreate = false; await submit.click();
    await expect(page).toHaveURL(/settings\/knowledge-base\/kb-configured$/);
    expect(created).toHaveLength(2);
    expect(created[1]).toEqual({ name: "Configured", description: "Selected before upload", defaultParsePreference: "Auto", indexSettings: { fulltext: true, vector: true, graph: true, embeddingDimension: 1024, embeddingModelCode: "PERSON_AI:EMBEDDING:bge-m3", graphModelCode: "PERSON_AI:ROUTE:ChatAuto", rerankModelCode: null, imageDescriptionModelCode: "vision-model" } });
    await page.goto(`${frontend!.baseUrl}/settings/rag`);
    await page.getByRole("button", { name: zh ? "Configured的操作" : "Actions for Configured", exact: true }).click();
    await page.getByRole("menuitem", { name: zh ? "配置索引" : "Configure indexes", exact: true }).click();
    const indexDialog = page.getByRole("dialog", { name: zh ? "配置索引 · Configured" : "Configure indexes · Configured", exact: true });
    await expect(indexDialog.getByRole("combobox", { name: zh ? "Embedding 模型" : "Embedding model", exact: true })).toContainText("BGE-M3 · Person-AI");
    await expect(indexDialog.getByRole("combobox", { name: zh ? "重排模型" : "Reranking model", exact: true })).toContainText(zh ? "关闭重排" : "Reranking off");
    await expect(indexDialog.getByRole("combobox", { name: zh ? "图片检索方式" : "Image retrieval", exact: true })).toContainText("Vision model");
  });
}

for (const locale of ["zh-CN", "en-US"] as const) {
  test(`Upload checks formats and shows legacy OCR before submission in ${locale}`, async ({ page }, info) => {
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    const zh = locale === "zh-CN";
    let submitted = 0; let checked = 0;
    await page.route("**/api/v1/knowledge-bases**", async route => {
      const request = route.request(); const path = new URL(request.url()).pathname;
      if (path.endsWith("/documents/preflight")) {
        checked++;
        expect(request.headers()["accept-language"]).toBe(locale);
        expect(request.postDataBuffer()!.toString()).toContain('filename="old.ppt"');
        return route.fulfill({json:{code:"0",message:"ok",data:{plan:{preference:"Auto",method:"Recognition",reason:"legacy_office",rulesVersion:"document-features-v1",features:{format:"ppt",localText:false,sourceSha256:"original"}}}}});
      }
      if (request.method() === "POST") submitted++;
      const data = path.endsWith("/parsing-capabilities")
        ? {defaultTier:"standard",defaultOcrMode:"Auto",availableTiers:["standard","advanced"],parserVersion:"test",officeTier:"flash"}
        : {id:"kb-preview",name:"Upload preview",description:"",documents:[],agentCodes:[],documentCount:0,defaultParsePreference:"Auto",defaultChunkConfig:{mode:"AUTO",maxChars:1200,overlapChars:120,headingDepth:2}};
      return route.fulfill({json:{code:"0",message:"ok",data}});
    });
    await page.goto(`${frontend!.baseUrl}/settings/knowledge-base/kb-preview`);
    const input = page.locator('input[type="file"]');
    await expect(input).toHaveAttribute("accept", /\.doc,.*\.ppt,.*\.xls,/);
    await expect(input).toHaveAttribute("accept", /\.epub,\.ofd,\.html/);
    await input.setInputFiles({name:"old.ppt",mimeType:"application/vnd.ms-powerpoint",buffer:Buffer.from("synthetic old Office")});
    const dialog = page.getByRole("dialog", {name:zh?"上传":"Upload",exact:true});
    await expect(dialog).toContainText("old.ppt");
    await expect(dialog).toContainText(zh?"扫描件识别":"OCR");
    await expect(dialog.getByRole("button",{name:zh?"上传":"Upload",exact:true})).toBeEnabled();
    expect(checked).toBe(1); expect(submitted).toBe(0);
    await page.screenshot({path:info.outputPath(`document-preflight-${locale}.png`),fullPage:true});
    await dialog.getByRole("button",{name:zh?"取消":"Cancel",exact:true}).and(page.locator("[data-slot=button]")).click();
    await expect(dialog).toHaveCount(0);
    expect(submitted).toBe(0);
  });
}
