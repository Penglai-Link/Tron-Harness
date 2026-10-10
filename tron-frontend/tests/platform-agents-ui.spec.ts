import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";
let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/settings/agents", { VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_AGENT_DATA_SOURCE: "api" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });
for (const locale of ["zh-CN", "en-US"] as const) {
  test(`Draft workspace and knowledge access inside the agent editor in ${locale}`, async ({ page }, testInfo) => {
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    const zh = locale === "zh-CN";
    const defaults = { builtIn:false, groupName:null as string | null, avatarId:"robot", description:"", rolePrompt:"", platformDefined:true, source:"PLATFORM_DEFAULT", toolAssignmentMode:"ALL_PLATFORM_TOOLS", mcpAccessMode:"DISABLED", toolExposureMode:"DOMAIN_DISCOVERY", expandedDomainPersistenceMode:"STICKY_EXPANDED", maxSteps:100,maxToolCalls:100,maxParallelToolCalls:4,selectedToolCodes:[],selectedServerCodes:[] };
    let agents = [{...defaults,builtIn:true,agentCode:"main-agent",name:"Main Agent"}, {...defaults,builtIn:true,agentCode:"builtin-inspector",name:"Builtin Inspector",groupName:"Builtins"}];
    let groups: string[] = [];
    const calls: string[] = [];
    let libraries: string[] = []; let spaces: string[] = []; let builtInSpaces: string[] = []; let failAuthorization = false;
    const ok = (data:unknown) => ({code:"0",message:"ok",data});
    await page.route("**/api/v1/tools**", route => route.fulfill({json:ok([])}));
    await page.route("**/api/v1/agents/**", async route => {
      const url = new URL(route.request().url());
      if (url.pathname.endsWith("/internal-groups")) {
        if (route.request().method() === "POST") {
          const name = route.request().postDataJSON().name.trim();
          groups = [...new Set([...groups, name])];
          return route.fulfill({json:ok({name})});
        }
        return route.fulfill({json:ok(groups)});
      }
      if (url.pathname.endsWith("/loops")) return route.fulfill({json:ok([])});
      expect(url.pathname).not.toContain("/workspaces/");
      if (url.pathname.endsWith("/workspace-grants")) {
        const builtin = url.pathname.includes("/builtin-inspector/");
        if (route.request().method() === "PUT") {
          if (failAuthorization) { failAuthorization = false; return route.fulfill({status:503,json:{code:"503",message:"Assignment unavailable",data:null}}); }
          if (builtin) builtInSpaces=route.request().postDataJSON().workspaceCodes;
          else spaces=route.request().postDataJSON().workspaceCodes;
          calls.push("authorize");
        }
        return route.fulfill({json:ok({workspaceCodes:builtin?builtInSpaces:spaces})});
      }
      if (url.pathname.endsWith("/system")) return route.fulfill({json:ok([{agentCode:"codex",name:"Codex",description:"System runtime",enabled:true,available:true,status:"ENABLED",source:"SYSTEM",lastScannedAt:null},{agentCode:"opencode",name:"OpenCode",description:"System runtime",enabled:true,available:true,status:"ENABLED",source:"SYSTEM",lastScannedAt:null}])});
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
    await expect(page.getByRole("region",{name:zh?"空间授权":"Workspace authorization",exact:true})).toHaveCount(0);
    await expect(definitions.getByRole("button",{name:zh?"查看 主智能体 · 内置智能体":"View Main Agent · Built-in agent",exact:true})).toBeVisible();
    const main=definitions.getByRole("button",{name:zh?"查看 主智能体 · 内置智能体":"View Main Agent · Built-in agent",exact:true});
    await main.click();
    const mainView=page.getByRole("dialog");
    await expect(mainView.getByRole("textbox",{name:zh?"名称":"Name",exact:true})).toBeDisabled();
    await expect(mainView.getByRole("button",{name:zh?/选择空间/:/Choose workspaces/})).toHaveCount(0);
    await expect(mainView.getByRole("button",{name:zh?"保存":"Save",exact:true})).toHaveCount(0);
    await mainView.getByRole("button",{name:zh?"取消":"Cancel",exact:true}).click();
    const shield=definitions.getByRole("button",{name:zh?"内置智能体":"Built-in agent",exact:true}).first();
    await shield.focus();
    await expect(page.getByRole("tooltip")).toContainText(zh?"内置智能体":"Built-in agent");
    await shield.blur();
    await expect(definitions.getByRole("textbox")).toHaveCount(0);
    await expect(definitions.getByRole("button",{name:zh?"分配空间":"Assign workspaces",exact:true})).toHaveCount(0);
    await definitions.getByRole("button",{name:zh?"创建":"New",exact:true}).click();
    const create=page.getByRole("dialog",{name:zh?"创建内部智能体":"New internal agent",exact:true});
    await create.getByRole("textbox",{name:zh?"编码":"Code",exact:true}).fill("reviewer");
    await create.getByRole("textbox",{name:zh?"名称":"Name",exact:true}).fill("Reviewer");
    await create.getByRole("button",{name:zh?"新增分组":"New group",exact:true}).click();
    await create.getByRole("textbox",{name:zh?"分组名称":"Group name",exact:true}).fill("Engineering");
    await create.getByRole("button",{name:zh?"添加":"Add",exact:true}).click();
    await expect(create.getByRole("combobox",{name:zh?"分组":"Group",exact:true})).toContainText("Engineering");
    expect(groups).toEqual([]); // New groups are drafts until the outer form saves.
    const avatarImages = create.locator("fieldset button img");
    await expect(avatarImages).toHaveCount(12);
    for (const image of await avatarImages.all()) {
      await expect(image).toHaveJSProperty("complete", true);
      expect(await image.evaluate((node: HTMLImageElement) => node.naturalWidth)).toBeGreaterThan(0);
    }
    await create.getByRole("button",{name:zh?"小龙":"Dragon",exact:true}).click();
    await expect(create.getByRole("img",{name:zh?"小龙":"Dragon",exact:true})).toBeVisible();
    await create.getByRole("button",{name:zh?"狐狸":"Fox",exact:true}).click();
    const preview = create.getByRole("img",{name:zh?"狐狸":"Fox",exact:true});
    await expect(preview).toBeVisible();
    const previewBounds = (await preview.boundingBox())!;
    expect(Math.abs(previewBounds.width - previewBounds.height)).toBeLessThan(1);
    const avatarRail = create.locator("fieldset [data-slot=scroll-area-viewport]");
    expect((await avatarRail.boundingBox())!.height).toBeCloseTo(previewBounds.height, 0);
    expect(await avatarRail.evaluate(node => node.scrollHeight > node.clientHeight)).toBe(true);
    await create.locator("fieldset").first().screenshot({animations:"disabled",path:testInfo.outputPath(`agent-avatars-${locale}.png`)});
    await page.setViewportSize({width:375,height:812});
    const narrowBounds = (await preview.boundingBox())!;
    expect(Math.abs(narrowBounds.width - narrowBounds.height)).toBeLessThan(1);
    expect(narrowBounds.x + narrowBounds.width).toBeLessThan(375);
    await create.locator("fieldset").first().screenshot({animations:"disabled",path:testInfo.outputPath(`agent-avatars-mobile-${locale}.png`)});
    await page.setViewportSize({width:1280,height:720});
    await create.getByRole("button",{name:zh?"创建":"New",exact:true}).click();
    await expect(create).toHaveCount(0);
    const card=definitions.getByRole("button",{name:zh?"编辑 Reviewer":"Edit Reviewer",exact:true});
    await expect(card).toBeVisible();
    expect(agents.find(agent => agent.agentCode === "reviewer")?.avatarId).toBe("fox");
    expect(agents.find(agent => agent.agentCode === "reviewer")?.groupName).toBe("Engineering");
    await page.screenshot({animations:"disabled",path:testInfo.outputPath(`agent-cards-${locale}.png`)});
    await card.click();
    const reviewer=page.getByRole("dialog",{name:zh?"编辑 Reviewer":"Edit Reviewer",exact:true});
    await reviewer.getByRole("textbox",{name:zh?"名称":"Name",exact:true}).fill("Reviewer draft");
    await page.mouse.click(8, 8);
    await expect(reviewer).toBeVisible();
    await expect(reviewer.getByRole("textbox",{name:zh?"名称":"Name",exact:true})).toHaveValue("Reviewer draft");
    expect(calls).toEqual(["define", "authorize"]);
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
    // Workspace selection is an editor draft, and canceling the editor discards it.
    await card.click();
    await reviewer.getByRole("button",{name:zh?/选择空间/:/Choose workspaces/}).click();
    const assign=page.getByRole("dialog").filter({has:page.getByRole("checkbox",{name:"Tron Core",exact:true})});
    await assign.getByRole("checkbox",{name:"Tron Core",exact:true}).check();
    await assign.getByRole("button",{name:zh?"确认":"Confirm",exact:true}).click();
    await expect(assign).toHaveCount(0);
    await expect(reviewer.getByText("Tron Core",{exact:true})).toBeVisible();
    expect(spaces).toEqual([]);
    await reviewer.getByRole("button",{name:zh?"新增分组":"New group",exact:true}).click();
    await reviewer.getByRole("textbox",{name:zh?"分组名称":"Group name",exact:true}).fill("Discarded group");
    await reviewer.getByRole("button",{name:zh?"添加":"Add",exact:true}).click();
    await reviewer.getByRole("button",{name:zh?"取消":"Cancel",exact:true}).click();
    expect(groups).toEqual(["Engineering"]);
    expect(agents.find(agent=>agent.agentCode==="reviewer")?.groupName).toBe("Engineering");
    await card.click();
    await expect(reviewer.getByRole("button",{name:zh?"选择空间（0）":"Choose workspaces (0)",exact:true})).toBeVisible();
    await reviewer.getByRole("button",{name:zh?/选择空间/:/Choose workspaces/}).click();
    await assign.getByRole("checkbox",{name:"Tron Core",exact:true}).check();
    await assign.getByRole("button",{name:zh?"确认":"Confirm",exact:true}).click();
    const fieldBounds = await reviewer.getByRole("button",{name:zh?/选择空间/:/Choose workspaces/}).boundingBox();
    const nameBounds = await reviewer.getByRole("textbox",{name:zh?"名称":"Name",exact:true}).boundingBox();
    expect(fieldBounds!.y).toBeGreaterThan(nameBounds!.y);
    expect(fieldBounds!.y - nameBounds!.y).toBeLessThan(120);
    await reviewer.getByRole("button",{name:zh?"新增分组":"New group",exact:true}).click();
    await reviewer.getByRole("textbox",{name:zh?"分组名称":"Group name",exact:true}).fill("Quality");
    await reviewer.getByRole("button",{name:zh?"添加":"Add",exact:true}).click();
    failAuthorization = true;
    await reviewer.getByRole("button",{name:zh?"保存":"Save",exact:true}).click();
    await expect(reviewer.getByRole("status")).toContainText("Assignment unavailable");
    await expect(reviewer.getByRole("combobox",{name:zh?"分组":"Group",exact:true})).toContainText("Quality");
    expect(spaces).toEqual([]);
    await reviewer.getByRole("button",{name:zh?"保存":"Save",exact:true}).click();
    await expect(reviewer).toHaveCount(0);
    expect(agents.find(agent=>agent.agentCode==="reviewer")?.groupName).toBe("Quality");
    expect(spaces).toEqual(["TRON_CORE"]);
    const quality=definitions.locator("details").filter({has:page.locator("summary",{hasText:"Quality"})});
    await expect(quality.getByRole("button",{name:zh?"编辑 Reviewer":"Edit Reviewer",exact:true})).toBeVisible();
    await quality.locator("summary").click();
    await expect(card).toBeHidden();
    await quality.locator("summary").click();
    await definitions.getByRole("combobox",{name:zh?"分组":"Group",exact:true}).click();
    await page.getByRole("option",{name:"Quality",exact:true}).click();
    await expect(main).toHaveCount(0);
    await expect(card).toBeVisible();
    await definitions.getByRole("combobox",{name:zh?"分组":"Group",exact:true}).click();
    await page.getByRole("option",{name:zh?"全部分组":"All groups",exact:true}).click();
    // A built-in identity other than the global main agent can still manage its independent grant.
    await definitions.getByRole("button",{name:zh?"查看 Builtin Inspector · 内置智能体":"View Builtin Inspector · Built-in agent",exact:true}).click();
    const builtin=page.getByRole("dialog",{name:zh?"查看 Builtin Inspector":"View Builtin Inspector",exact:true});
    await expect(builtin.getByRole("textbox",{name:zh?"名称":"Name",exact:true})).toBeDisabled();
    await expect(builtin.getByRole("combobox",{name:zh?"分组":"Group",exact:true})).toBeDisabled();
    await builtin.getByRole("button",{name:zh?/选择空间/:/Choose workspaces/}).click();
    await assign.getByRole("checkbox",{name:"Tron Core",exact:true}).check();
    await assign.getByRole("button",{name:zh?"确认":"Confirm",exact:true}).click();
    await expect(page.locator('[data-slot="toast-description"]')).toHaveCount(0, {timeout:10000});
    await page.screenshot({animations:"disabled",path:testInfo.outputPath(`builtin-authorization-${locale}.png`)});
    await builtin.getByRole("button",{name:zh?"保存":"Save",exact:true}).click();
    await expect(builtin).toHaveCount(0);
    expect(builtInSpaces).toEqual(["TRON_CORE"]);
    await expect(page.locator('[data-slot="toast-description"]')).toHaveCount(0, {timeout:10000});
    await page.setViewportSize({width:1280,height:1200});
    await page.screenshot({animations:"disabled",fullPage:true,path:testInfo.outputPath(`agent-management-${locale}.png`)});
    await page.setViewportSize({width:375,height:812});
    await expect(definitions.getByRole("button",{name:zh?"编辑 Reviewer":"Edit Reviewer",exact:true})).toBeVisible();
    expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(375);
    await page.screenshot({animations:"disabled",fullPage:true,path:testInfo.outputPath(`agent-management-mobile-${locale}.png`)});
  });
}
