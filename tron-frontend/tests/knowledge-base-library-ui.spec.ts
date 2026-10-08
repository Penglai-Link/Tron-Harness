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
