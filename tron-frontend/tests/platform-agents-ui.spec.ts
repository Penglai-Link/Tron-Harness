import builtinLoops from "../../shared/agent-loops.json" with { type: "json" };
import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";
let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/settings/agents", { VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_AGENT_DATA_SOURCE: "api" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });
for (const locale of ["zh-CN", "en-US"] as const) {
  test(`Define, bind knowledge, then authorize workspaces in ${locale}`, async ({ page }, testInfo) => {
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    const zh = locale === "zh-CN";
    const defaults = { avatarId:"robot", description:"", rolePrompt:"", platformDefined:true, source:"PLATFORM_DEFAULT", toolAssignmentMode:"ALL_PLATFORM_TOOLS", mcpAccessMode:"DISABLED", toolExposureMode:"DOMAIN_DISCOVERY", expandedDomainPersistenceMode:"STICKY_EXPANDED", maxSteps:100,maxToolCalls:100,maxParallelToolCalls:4,selectedToolCodes:[],selectedServerCodes:[] };
    let agents = [{...defaults,agentCode:"main-agent",name:"Main Agent"}];
    const calls: string[] = [];
    let libraries: string[] = []; let spaces: string[] = [];
    const ok = (data:unknown) => ({code:"0",message:"ok",data});
    await page.route("**/api/v1/tools**", route => route.fulfill({json:ok([])}));
    await page.route("**/api/v1/agents/**", async route => {
      const url = new URL(route.request().url());
      if (url.pathname.endsWith("/loops")) return route.fulfill({json:ok(builtinLoops)});
      expect(url.pathname).not.toContain("/workspaces/");
      if (url.pathname.endsWith("/workspace-grants")) {
        if (route.request().method() === "PUT") { spaces=route.request().postDataJSON().workspaceCodes; calls.push("authorize"); }
        return route.fulfill({json:ok({workspaceCodes:spaces})});
      }
      if (url.pathname.endsWith("/system")) return route.fulfill({json:ok([{agentCode:"codex",name:"Codex",description:"System runtime",enabled:true,available:true,status:"ENABLED",source:"SYSTEM",lastScannedAt:null}])});
      if (route.request().method() === "PATCH") {
        const code = url.pathname.split("/").at(-2);
        agents = agents.map(agent => agent.agentCode === code ? {...agent, ...route.request().postDataJSON()} : agent);
        calls.push("edit");
        return route.fulfill({json:ok(agents.find(agent => agent.agentCode === code))});
      }
      if (route.request().method() === "POST") {
        const payload = route.request().postDataJSON();
        expect(url.pathname).toBe("/api/v1/agents/registry/internal");
        const created = {...defaults,...payload}; agents=[...agents,created]; calls.push("define");
        return route.fulfill({json:ok(created)});
      }
      return route.fulfill({json:ok(agents)});
    });
    await page.route("**/api/v1/admin/workspaces/options*", route => route.fulfill({json:ok([{workspaceCode:"TRON_CORE",name:"Tron Core",label:"Tron Core"}])}));
    await page.route("**/api/v1/knowledge-bases/agent-grants/**", async route => {
      const url = new URL(route.request().url()); expect(url.searchParams.has("workspaceCode")).toBe(false);
      if (route.request().method() === "PUT") { expect(route.request().postDataJSON()).toEqual({libraryIds:["kb-shared"]}); libraries=["kb-shared"]; calls.push("knowledge"); return route.fulfill({json:ok({})}); }
      return route.fulfill({json:ok([{id:"kb-shared",name:"Shared library",description:"",documentCount:2,granted:url.pathname.endsWith("reviewer") && libraries.length>0}])});
    });
    await page.goto(`${frontend!.baseUrl}/settings/agents`);
    await expect(page.getByRole("tab")).toHaveCount(2);
    const definitions=page.getByRole("region",{name:zh?"智能体定义":"Agent definitions",exact:true});
    const authorization=page.getByRole("region",{name:zh?"空间授权":"Workspace authorization",exact:true});
    await expect(definitions.getByRole("button",{name:zh?"编辑 主智能体":"Edit Main Agent",exact:true})).toBeVisible();
    await expect(definitions.getByRole("textbox")).toHaveCount(0);
    await expect(definitions.getByRole("button",{name:zh?"分配空间":"Assign workspaces",exact:true})).toHaveCount(0);
    await expect(authorization.getByRole("textbox")).toHaveCount(0);
    await expect(authorization.getByRole("button",{name:zh?"创建":"New",exact:true})).toHaveCount(0);
    await definitions.getByRole("button",{name:zh?"创建":"New",exact:true}).click();
    const create=page.getByRole("dialog",{name:zh?"创建内部智能体":"New internal agent",exact:true});
    await create.getByRole("textbox",{name:zh?"编码":"Code",exact:true}).fill("reviewer");
    await create.getByRole("textbox",{name:zh?"名称":"Name",exact:true}).fill("Reviewer");
    const avatarImages = create.locator("fieldset img");
    await expect(avatarImages).toHaveCount(12);
    for (const image of await avatarImages.all()) {
      await expect(image).toHaveJSProperty("complete", true);
      expect(await image.evaluate((node: HTMLImageElement) => node.naturalWidth)).toBeGreaterThan(0);
    }
    await page.screenshot({animations:"disabled",path:testInfo.outputPath(`agent-avatars-${locale}.png`)});
    await create.getByRole("button",{name:zh?"狐狸":"Fox",exact:true}).click();
    await create.getByRole("button",{name:zh?"创建":"New",exact:true}).click();
    await expect(create).toHaveCount(0);
    const card=definitions.getByRole("button",{name:zh?"编辑 Reviewer":"Edit Reviewer",exact:true});
    await expect(card).toBeVisible();
    expect(agents.find(agent => agent.agentCode === "reviewer")?.avatarId).toBe("fox");
    await page.screenshot({animations:"disabled",path:testInfo.outputPath(`agent-cards-${locale}.png`)});
    await card.click();
    const reviewer=page.getByRole("dialog",{name:zh?"编辑 Reviewer":"Edit Reviewer",exact:true});
    await reviewer.getByRole("textbox",{name:zh?"名称":"Name",exact:true}).fill("Reviewer draft");
    await page.mouse.click(8, 8);
    await expect(reviewer).toBeVisible();
    await expect(reviewer.getByRole("textbox",{name:zh?"名称":"Name",exact:true})).toHaveValue("Reviewer draft");
    expect(calls).toEqual(["define"]);
    await reviewer.getByRole("textbox",{name:zh?"名称":"Name",exact:true}).fill("Reviewer");
    await reviewer.getByRole("button",{name:zh?/选择知识库/:/Choose knowledge bases/}).click();
    await page.getByRole("switch",{name:"Shared library",exact:true}).check();
    const knowledge=page.getByRole("dialog").filter({has:page.getByRole("switch",{name:"Shared library",exact:true})});
    await knowledge.getByRole("button",{name:zh?"确认":"Confirm",exact:true}).click();
    await expect(knowledge).toHaveCount(0); expect(spaces).toEqual([]);
    expect(libraries).toEqual([]);
    await page.screenshot({animations:"disabled",path:testInfo.outputPath(`agent-editor-${locale}.png`)});
    await reviewer.getByRole("button",{name:zh?"保存":"Save",exact:true}).click();
    await expect(reviewer).toHaveCount(0);
    expect(libraries).toEqual(["kb-shared"]);
    await authorization.getByRole("row").filter({hasText:"Reviewer"}).getByRole("button",{name:zh?"分配空间":"Assign workspaces",exact:true}).click();
    const assign=page.getByRole("dialog").filter({has:page.getByRole("checkbox",{name:"Tron Core",exact:true})});
    await assign.getByRole("checkbox",{name:"Tron Core",exact:true}).check();
    await assign.getByRole("button",{name:zh?"保存":"Save",exact:true}).click();
    await expect(assign).toHaveCount(0); expect(calls).toEqual(["define","knowledge","authorize"]); expect(spaces).toEqual(["TRON_CORE"]);
    await authorization.scrollIntoViewIfNeeded();
    await page.screenshot({animations:"disabled",path:testInfo.outputPath(`agent-management-${locale}.png`)});
  });
}
