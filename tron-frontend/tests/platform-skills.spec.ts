import { expect, test } from "@playwright/test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  spawnManagedProcess,
  stopManagedProcess,
  type ManagedProcess,
} from "./support/managed-process";
import {
  routeActiveProfileApi,
  seedResourceUiShell,
  startResourceUiFrontend,
  stopResourceUiFrontend,
  type ResourceUiFrontend,
} from "./support/resource-ui-harness";

const longSkillName = `browser${"x".repeat(90)}`;
let directory = "",
  api = "";
let backend: ManagedProcess | undefined,
  frontend: ResourceUiFrontend | undefined;
async function request(path: string, method = "GET", body?: unknown) {
  const response = await fetch(`${api}/api/v1/${path}`, {
    method,
    headers: {
      Authorization: "Bearer profile-test-token",
      "Content-Type": "application/json",
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  expect(response.ok, await response.clone().text()).toBeTruthy();
  return (await response.json()).data;
}
test.beforeAll(async () => {
  test.setTimeout(600_000);
  directory = await mkdtemp(join(tmpdir(), "tron-platform-skills-"));
  const skill = join(directory, "skills/.agents/skills/browser-business");
  await mkdir(skill, { recursive: true });
  await writeFile(
    join(skill, "SKILL.md"),
    "---\nname: browser-business\ndescription: Browser business skill\n---\nBusiness instructions"
  );
  const longSkill = join(directory, "skills/.agents/skills", longSkillName);
  await mkdir(longSkill, { recursive: true });
  await writeFile(
    join(longSkill, "SKILL.md"),
    `---\nname: ${longSkillName}\ndescription: Narrow-screen layout fixture\n---\nInstructions`
  );
  const port = await new Promise<number>((done, reject) => {
    const server = createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string")
        throw new Error("missing port");
      server.close(() => done(address.port));
    });
  });
  api = `http://127.0.0.1:${port}`;
  let logs = "";
  backend = spawnManagedProcess("rtk", ["make", "be-ui-acceptance"], {
    cwd: resolve(fileURLToPath(import.meta.url), "../../.."),
    env: {
      ...process.env,
      CARGO_NET_OFFLINE: "true",
      TRON_ACCEPTANCE_DIRECTORY: directory,
      TRON_ACCEPTANCE_PORT: String(port),
    },
    stdio: "pipe",
  });
  backend.stdout.on(
    "data",
    (chunk) => (logs = (logs + chunk.toString()).slice(-16000))
  );
  backend.stderr.on(
    "data",
    (chunk) => (logs = (logs + chunk.toString()).slice(-16000))
  );
  let ready = false;
  for (const deadline = Date.now() + 570_000; Date.now() < deadline; ) {
    if (backend.exitCode !== null) throw new Error(logs);
    try {
      ready = (await fetch(`${api}/api/v1/health`)).ok;
    } catch {
      /* owned startup */
    }
    if (ready) break;
    await new Promise((done) => setTimeout(done, 250));
  }
  if (!ready) throw new Error(logs);
  frontend = await startResourceUiFrontend("/settings/skills", {
    VITE_TRON_DATA_SOURCE: "mock",
    VITE_TRON_AGENT_DATA_SOURCE: "api",
    VITE_TRON_MCP_DATA_SOURCE: "api",
    VITE_TRON_API_BASE_URL: "",
    TRON_FRONTEND_API_PROXY_TARGET: api,
  });
});
test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  await stopManagedProcess(backend);
  if (directory) await rm(directory, { recursive: true, force: true });
});

test("platform skills, bidirectional drafts and disabled resource focus hints use real API", async ({
  page,
}, testInfo) => {
  test.setTimeout(90_000);
  await seedResourceUiShell(page, "en-US");
  await routeActiveProfileApi(page);
  await request("rag/corpora/skill/settings", "PUT", {
    fulltext: true,
    vector: false,
    graph: false,
  });
  const initial = await request("agents/registry/internal/main-agent/skills");
  const builtin = initial.find(
    (s: { providerPackId: string; assigned: boolean }) =>
      s.providerPackId === "tron.harness" && s.assigned
  );
  expect(builtin).toBeTruthy();
  expect(
    initial.find(
      (s: { skillName: string }) => s.skillName === "browser-business"
    ).assigned
  ).toBe(false);
  await page.goto(`${frontend!.baseUrl}/settings/skills`);
  await page
    .getByRole("searchbox", { name: "Search skills" })
    .fill("browser-business");
  const row = page.getByRole("row").filter({ hasText: "browser-business" });
  await row.getByRole("button", { name: "Assign Agents" }).click();
  const grant = page.getByRole("dialog");
  const mainGrant = grant
    .locator("label")
    .filter({ hasText: "main-agent" })
    .getByRole("checkbox");
  await mainGrant.check();
  const assignmentSaved = page.waitForResponse(r => r.request().method() === "PUT" && r.url().includes("/browser-business/agent-grants"));
  await grant.getByRole("button", { name: "Save", exact: true }).click();
  const assignment = await assignmentSaved;
  expect(assignment.request().postDataJSON()).toEqual({agentCodes:["main-agent"]});
  expect(assignment.status(), await assignment.text()).toBe(200);
  expect((await request("skills/registry/browser-business/agent-grants")).agentCodes).toContain("main-agent");
  await expect(grant).toHaveCount(0);
  await expect.poll(async () => (await request("agents/registry/internal/main-agent/skills")).find((s: { skillName: string }) => s.skillName === "browser-business").assigned).toBe(true);
  await page.goto(`${frontend!.baseUrl}/settings/agents`);
  const edit = page
    .getByRole("button", { name: /(?:Edit|View) Main Agent/ })
    .first();
  await edit.click();
  const editor = page.getByRole("dialog").first();
  await editor.getByRole("tab", { name: "Capabilities", exact: true }).click();
  await editor.getByRole("button", { name: /Select skills/ }).click();
  const selection = page.getByRole("dialog", {
    name: "Select skills",
    exact: true,
  });
  await selection
    .getByRole("checkbox", { name: "browser-business", exact: true })
    .uncheck();
  await selection.getByRole("button", { name: "Confirm selection" }).click();
  await editor.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(
    (await request("agents/registry/internal/main-agent/skills")).find(
      (s: { skillName: string }) => s.skillName === "browser-business"
    ).assigned
  ).toBe(true);
  await edit.click();
  await editor.getByRole("tab", { name: "Capabilities", exact: true }).click();
  await editor.getByRole("button", { name: /Select skills/ }).click();
  await selection
    .getByRole("checkbox", { name: "browser-business", exact: true })
    .uncheck();
  await selection.getByRole("button", { name: "Confirm selection" }).click();
  await editor.getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor).toHaveCount(0, { timeout: 20_000 });
  expect(
    (await request(`skills/registry/browser-business/agent-grants`)).agentCodes
  ).not.toContain("main-agent");
  await request("capabilities/packs/tron.harness/enabled", "PUT", {
    enabled: false,
  });
  await page.reload();
  await edit.click();
  await editor.getByRole("tab", { name: "Capabilities", exact: true }).click();
  await editor.getByRole("button", { name: /Select skills/ }).click();
  const blocked = selection.getByRole("checkbox", {
    name: builtin.skillName,
    exact: true,
  });
  await expect(blocked).toBeDisabled();
  await expect(blocked).not.toBeChecked();
  const reason = selection
    .getByRole("button", {
      name: "Capability pack tron.harness is disabled",
      exact: true,
    })
    .first();
  await reason.focus();
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await reason.hover();
  await expect(page.getByRole("tooltip")).toContainText(
    "Capability pack tron.harness is disabled"
  );
  await reason.blur();
  await page.mouse.move(0,0);
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await reason.hover();
  await expect(page.getByRole("tooltip")).toContainText(
    "Capability pack tron.harness is disabled"
  );
  await selection
    .getByRole("button", { name: "Select all", exact: true })
    .click();
  await expect(blocked).not.toBeChecked();
  await page.screenshot({
    path: testInfo.outputPath("disabled-skills.png"),
    fullPage: true,
  });
  await selection.getByRole("button", { name: "Cancel", exact: true }).click();
  await editor.getByRole("button", { name: "Cancel", exact: true }).click();
  await request("capabilities/packs/tron.harness/enabled", "PUT", {
    enabled: true,
  });
  const enabled = await request("agents/registry/internal/main-agent/skills");
  expect(
    enabled.find(
      (s: { skillName: string }) => s.skillName === builtin.skillName
    ).assigned
  ).toBe(true);
  await request(`skills/registry/${builtin.skillName}/agent-grants`, "PUT", {
    agentCodes: [],
  });
  await request("capabilities/packs/tron.harness/enabled", "PUT", {
    enabled: true,
  });
  expect(
    (await request(`skills/registry/${builtin.skillName}/agent-grants`))
      .agentCodes
  ).toEqual([]);
  await page.goto(`${frontend!.baseUrl}/settings/skills`);
  await expect(page.locator('a[href="/skills"]')).toHaveCount(0);
  const legacy = await fetch(
    `${api}/api/v1/skills/workspaces/TRON_CORE/bindings`,
    { headers: { Authorization: "Bearer profile-test-token" } }
  );
  expect(legacy.status).toBe(404);
});

for (const variant of [
  { locale: "en-US", theme: "light", width: 1280 },
  { locale: "zh-CN", theme: "matrix-dark", width: 1280 },
  { locale: "en-US", theme: "matrix-dark", width: 390 },
  { locale: "zh-CN", theme: "light", width: 390 },
] as const) {
  test(`skills UI ${variant.locale} ${variant.theme} ${variant.width}px`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(60_000);
    const zh = variant.locale === "zh-CN";
    await request("system/skins/active", "PUT", { id: variant.theme });
    await page.setViewportSize({ width: variant.width, height: 844 });
    await seedResourceUiShell(page, variant.locale);
    await routeActiveProfileApi(page);
    await page.addInitScript((theme) => {
      localStorage.setItem("tron.theme", theme);
      localStorage.setItem("tron.motion.mode", "off");
    }, variant.theme);
    await page.goto(`${frontend!.baseUrl}/settings/skills`);
    await expect(page.locator("html")).toHaveAttribute("lang", variant.locale);
    await expect(page.locator("html")).toHaveAttribute(
      "data-theme",
      variant.theme
    );
    await page
      .getByRole("searchbox", { name: zh ? "搜索技能" : "Search skills" })
      .fill("browser-business");
    await expect(
      page.getByRole("row").filter({ hasText: "browser-business" })
    ).toBeVisible();
    await page.goto(`${frontend!.baseUrl}/settings/agents`);
    await page
      .getByRole("button", {
        name: zh ? /(?:编辑|查看).*主智能体/ : /(?:Edit|View) Main Agent/,
      })
      .first()
      .click();
    const editor = page.getByRole("dialog").first();
    await editor.getByRole("tab", { name: zh ? "能力资源" : "Capabilities", exact: true }).click();
    const choose = editor.getByRole("button", {
      name: zh ? /选择技能/ : /Select skills/,
    });
    await choose.click();
    const selection = page.getByRole("dialog", {
      name: zh ? "选择技能" : "Select skills",
      exact: true,
    });
    await expect(page.locator(`style[data-tron-skin="${variant.theme}"]`)).toHaveCount(1);
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    const search = selection.getByRole("searchbox", {
      name: zh ? "搜索技能" : "Search skills",
    });
    await search.fill(longSkillName);
    await expect(
      selection.getByRole("checkbox", { name: longSkillName })
    ).toBeVisible();
    await expect
      .poll(() =>
        selection.evaluate(
          (element) => element.scrollWidth - element.clientWidth
        )
      )
      .toBeLessThanOrEqual(1);
    await expect(selection.getByRole("checkbox")).toHaveCount(1);
    await expect
      .poll(() =>
        selection
          .locator("label")
          .evaluate((element) => element.scrollWidth - element.clientWidth)
      )
      .toBeLessThanOrEqual(1);
    const bounds = await selection.boundingBox();
    expect(bounds!.x).toBeGreaterThanOrEqual(0);
    expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(variant.width);
    await page.screenshot({
      path: testInfo.outputPath("skill-picker-layout.png"),
      fullPage: true,
    });
    await selection
      .getByRole("button", { name: zh ? "取消" : "Cancel", exact: true })
      .click();
    await expect(choose).toBeFocused();
    await editor
      .getByRole("button", { name: zh ? "取消" : "Cancel", exact: true })
      .click();
  });
}

test.afterEach(async ({page}, info) => {
  if (info.status !== info.expectedStatus) {
    const path = info.outputPath("failed-page.txt");
    await (await import("node:fs/promises")).writeFile(path, await page.locator("body").ariaSnapshot());
    await info.attach("failed-page", {path, contentType: "text/plain"});
  }
});
