import { test, expect } from "@playwright/test";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/settings/agents", { VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_AGENT_DATA_SOURCE: "api" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const locale of ["zh-CN", "en-US"] as const) {
  test(`System avatar grid keeps scan, policy and workspace access in ${locale}`, async ({ page }, testInfo) => {
    const zh = locale === "zh-CN";
    await routeResourceUiShellApi(page);
    await seedResourceUiShell(page, locale);
    const ok = (data: unknown) => ({ code: "0", message: "ok", data });
    const base = { description: "System runtime", source: "SYSTEM", lastScannedAt: "2026-10-10T02:00:00Z", models: [] };
    let records = [
      { ...base, agentCode: "codex", name: "Codex", enabled: true, available: true, status: "ENABLED" },
      { ...base, agentCode: "opencode", name: "OpenCode", enabled: false, available: false, status: "UNAVAILABLE" },
    ];
    let assigned: string[] = [];
    const calls: string[] = [];
    await page.route("**/api/v1/tools**", route => route.fulfill({ json: ok([]) }));
    await page.route("**/api/v1/admin/workspaces/options*", route => route.fulfill({ json: ok([{ workspaceCode: "TRON_CORE", name: "Tron Core", label: "Tron Core" }]) }));
    await page.route("**/api/v1/agents/**", async route => {
      const url = new URL(route.request().url());
      expect(url.pathname).not.toContain("/workspaces/");
      if (url.pathname.endsWith("/loops")) return route.fulfill({ json: ok([]) });
      if (url.pathname.endsWith("/workspace-grants")) {
        if (route.request().method() === "PUT") { assigned = route.request().postDataJSON().workspaceCodes; calls.push("authorize"); }
        return route.fulfill({ json: ok({ workspaceCodes: assigned }) });
      }
      if (url.pathname.endsWith("/rescan")) {
        calls.push("scan");
        records = records.map(record => ({ ...record, lastScannedAt: "2026-10-10T03:00:00Z" }));
        return route.fulfill({ json: ok({ scannedCount: 2, availableCount: 1, unavailableCount: 1, records }) });
      }
      if (url.pathname.endsWith("/disable") || url.pathname.endsWith("/enable")) {
        const enabled = url.pathname.endsWith("/enable");
        calls.push(enabled ? "enable" : "disable");
        records = records.map(record => record.agentCode === "codex" ? { ...record, enabled, status: enabled ? "ENABLED" : "DISABLED" } : record);
        return route.fulfill({ json: ok(records[0]) });
      }
      return route.fulfill({ json: ok(url.pathname.endsWith("/system") ? records : []) });
    });
    await page.goto(`${frontend!.baseUrl}/settings/agents`);
    const section = page.getByRole("region", { name: zh ? "系统智能体" : "System Agents", exact: true });
    await expect(section.getByRole("table")).toHaveCount(0);
    const codex = section.getByRole("button", { name: zh ? "管理 Codex" : "Manage Codex", exact: true });
    const openCode = section.getByRole("button", { name: zh ? "管理 OpenCode" : "Manage OpenCode", exact: true });
    await expect(codex).toBeVisible();
    await expect(openCode).toBeVisible();
    for (const [button, file] of [[codex, "codex-pixel.png"], [openCode, "opencode-pixel.png"]] as const) {
      const avatar = button.locator("img");
      await expect(avatar).toHaveAttribute("src", `/agent-brands/${file}`);
      await expect(avatar).toHaveJSProperty("complete", true);
      expect(await avatar.evaluate((node: HTMLImageElement) => node.naturalWidth)).toBeGreaterThanOrEqual(1024);
      expect(await avatar.evaluate(node => parseFloat(getComputedStyle(node.parentElement!).borderRadius))).toBeGreaterThanOrEqual(40);
      expect(await avatar.evaluate(node => getComputedStyle(node).imageRendering)).toBe("pixelated");
    }
    await section.screenshot({ animations: "disabled", path: testInfo.outputPath(`system-grid-${locale}.png`) });
    await section.getByRole("button", { name: zh ? "扫描" : "Scan", exact: true }).click();
    await expect.poll(() => calls).toEqual(["scan"]);
    await codex.click();
    const detail = page.getByRole("dialog", { name: "Codex", exact: true });
    await expect(detail.getByText(zh ? "系统扫描" : "System Scan", { exact: true })).toBeVisible();
    await expect(detail.getByRole("switch")).toBeEnabled();
    await detail.getByRole("switch").click();
    await expect(detail.getByRole("switch")).toBeEnabled();
    await detail.getByRole("switch").click();
    await expect(detail.getByRole("switch")).toBeEnabled();
    await detail.getByRole("button", { name: zh ? "分配空间" : "Assign workspaces", exact: true }).click();
    const assign = page.getByRole("dialog").filter({ has: page.getByRole("checkbox", { name: "Tron Core", exact: true }) });
    await assign.getByRole("checkbox", { name: "Tron Core", exact: true }).check();
    await assign.getByRole("button", { name: zh ? "保存" : "Save", exact: true }).click();
    await expect(assign).toHaveCount(0);
    expect(assigned).toEqual(["TRON_CORE"]);
    expect(calls).toEqual(["scan", "disable", "enable", "authorize"]);
    await detail.screenshot({ animations: "disabled", path: testInfo.outputPath(`system-detail-${locale}.png`) });
    await detail.getByRole("button", { name: zh ? "关闭" : "Close", exact: true }).click();
    await openCode.click();
    const unavailable = page.getByRole("dialog", { name: "OpenCode", exact: true });
    await expect(unavailable.getByRole("switch")).toBeDisabled();
    await unavailable.getByRole("button", { name: zh ? "关闭" : "Close", exact: true }).click();
    await expect(codex).toBeVisible(); await expect(openCode).toBeVisible();
    await page.setViewportSize({ width: 375, height: 812 });
    await section.screenshot({ animations: "disabled", path: testInfo.outputPath(`system-grid-mobile-${locale}.png`) });
    const bounds = (await section.boundingBox())!;
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(375);
  });
}

// Keep the rendered state beside failed acceptance evidence.
test.afterEach(async ({page}, info) => {
  if (info.status !== info.expectedStatus) {
    const path = info.outputPath("failed-page.txt");
    await (await import("node:fs/promises")).writeFile(path, await page.locator("body").ariaSnapshot());
    await info.attach("failed-page", {path, contentType: "text/plain"});
  }
});
