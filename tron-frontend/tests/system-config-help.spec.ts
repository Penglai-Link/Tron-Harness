import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { parse } from "yaml";
import { test, expect } from "@playwright/test";
import { createInstance } from "i18next";
import { pages as en } from "../src/i18n/locales/en-US/pages";
import { pages as zh } from "../src/i18n/locales/zh-CN/pages";
import { systemConfigHelp } from "../src/domain/settings/model/systemConfigHelp";
import type { SystemConfigItem } from "../src/domain/settings/model/SystemConfig";
import { routeResourceUiShellApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

// Exercise the shipped configuration, rather than a list copied from help keys.
const config = parse(readFileSync(fileURLToPath(new URL("../../tron-backend/application.yaml", import.meta.url)), "utf8"));
function flatten(value: unknown, path = ""): SystemConfigItem[] {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    return Object.entries(value).flatMap(([key, child]) => flatten(child, path ? `${path}.${key}` : key));
  }
  return [{ path, value: value as SystemConfigItem["value"], valueType: Array.isArray(value) ? "array" : typeof value === "boolean" ? "boolean" : typeof value === "number" ? "number" : "string", effect: path.startsWith("modelInvocation.") ? "hotUpdate" : "restart", sensitive: path === "variableCrypto.keyBase64", overriddenByEnvironment: false }];
}
const items = flatten(config).filter(item => !["mineru.parseEndpoint", "mineru.backend", "mineru.parseMethod", "mineru.returnContentList"].includes(item.path));
// These default fields are materialized by the system-config API even when YAML
// omits them, including prices and an unset absolute tool-schema token cap.
items.push(...flatten({ shortTermMemory: { context: { blocks: { toolSchema: { maxTokens: null } }, providers: { prices: { input: null, cachedInput: null, output: null, currency: null } } } } }));
items.push(...flatten({ resource: { storage: { profiles: { "custom-s3": { type: "s3", endpoint: "https://storage.example", region: "us-east-1", bucket: "tron", prefix: "files", forcePathStyle: true, accessKeyIdEnv: "S3_ACCESS", secretAccessKeyEnv: "S3_SECRET", sessionTokenEnv: null } } } } }));

let frontend: ResourceUiFrontend | undefined;
test.beforeAll(async () => { frontend = await startResourceUiFrontend("/settings/general", { VITE_TRON_DATA_SOURCE: "mock" }); });
test.afterAll(async () => { await stopResourceUiFrontend(frontend); });

for (const locale of ["en-US", "zh-CN"] as const) {
  test(`Every shipped configuration field has scoped help in ${locale}`, async ({ page }, testInfo) => {
    const i18n = createInstance();
    // No fallback language: a missing translation must fail coverage.
    await i18n.init({ lng: locale, fallbackLng: false, resources: { [locale]: { pages: locale === "en-US" ? en : zh } } });
    const t = i18n.getFixedT(locale, "pages");
    for (const item of items) {
      const description = systemConfigHelp(item.path, t);
      expect(description, item.path).not.toBe("");
      expect(description, item.path).not.toMatch(/设置此后端配置值|Sets this backend configuration value|settingsGeneral\.systemConfig/);
    }
    await routeResourceUiShellApi(page); await seedResourceUiShell(page, locale);
    let currentItems = items;
    await page.route("**/api/v1/system/config", route => route.fulfill({ json: { code: "SUCCESS", message: "ok", data: { revision: "help-test", path: "application.yaml", items: currentItems } } }));
    await page.goto(`${frontend!.baseUrl}/settings/general`);
    await page.getByRole("button", { name: locale === "zh-CN" ? "后端配置" : "Backend configuration", exact: true }).click();
    await page.getByRole("button", { name: /MinerU/ }).click();
    const help = (path: string) => page.getByRole("button", { name: t("settingsGeneral.systemConfig.fieldHelp", { path }), exact: true });
    const tooltip = page.locator('[role="tooltip"][data-open]');
    await help("mineru.defaultTier").hover();
    await expect(tooltip).toContainText("flash");
    await expect(tooltip).toContainText("advanced");
    await expect(tooltip).toContainText(locale === "zh-CN" ? "扫描 PDF" : "scanned-PDF");
    await page.screenshot({ animations: "disabled", path: testInfo.outputPath(`config-help-${locale}.png`) });
    await help("mineru.timeoutSeconds").hover();
    await expect(tooltip).toContainText(locale === "zh-CN" ? "20 分钟" : "20 minutes");
    await help("mineru.imageDescription.timeoutSeconds").hover();
    await expect(tooltip).toContainText(locale === "zh-CN" ? "图片说明" : "image-description");
    await expect(tooltip).not.toContainText("1200");
    await page.mouse.move(1, 1);
    await help("mineru.ocrMode").focus();
    await expect(tooltip).toContainText("auto");
    await expect(tooltip).toContainText("txt");
    await expect(tooltip).toContainText("ocr");
    expect(systemConfigHelp("resource.storage.profiles.another-local.root", t)).toContain(locale === "zh-CN" ? "文件资源" : "file-resource");
    expect(systemConfigHelp("navigation.maxDepth", t)).not.toBe(systemConfigHelp("resource.limits.maxDepth", t));
    expect(systemConfigHelp("eventBus.enabled", t)).not.toBe(systemConfigHelp("mineru.enabled", t));
    expect(systemConfigHelp("future.unsupported.field", t)).toBe("");
    await page.mouse.move(1, 1);
    await page.getByRole("button", { name: locale === "zh-CN" ? /模型调用/ : /Model invocation/ }).click();
    await expect(page.getByRole("spinbutton", { name: "modelInvocation.embedding.maxBatchInputs", exact: true })).toHaveValue("32");
    await expect(page.getByRole("spinbutton", { name: "modelInvocation.embedding.maxBatchTokens", exact: true })).toHaveValue("32768");
    await help("modelInvocation.embedding.maxBatchTokens").hover();
    await expect(tooltip).toContainText("UTF-8");
    await page.mouse.move(1, 1);
    await page.getByRole("spinbutton", { name: "modelInvocation.embedding.maxBatchInputs", exact: true }).fill("8");
    await page.getByRole("spinbutton", { name: "modelInvocation.embedding.callTimeoutSeconds", exact: true }).fill("240");
    let submitted: unknown;
    await page.route("**/api/v1/system/config", async route => {
      if (route.request().method() === "PUT") {
        const body = route.request().postDataJSON() as { changes: { path: string; value: number }[] };
        submitted = body;
        currentItems = currentItems.map(item => ({ ...item, value: body.changes.find(change => change.path === item.path)?.value ?? item.value }));
        await route.fulfill({ json: { code: "SUCCESS", message: "ok", data: { revision: "saved", changedPaths: ["modelInvocation.embedding.maxBatchInputs", "modelInvocation.embedding.callTimeoutSeconds"], hotUpdatePaths: ["modelInvocation.embedding.maxBatchInputs", "modelInvocation.embedding.callTimeoutSeconds"], restartPaths: [], migrationPaths: [] } } });
      } else await route.fallback();
    });
    await page.getByRole("button", { name: t("settingsGeneral.systemConfig.actions.save", { count: 2 }), exact: true }).click();
    await expect.poll(() => submitted).toEqual({ revision: "help-test", confirmMigration: false, changes: expect.arrayContaining([{ path: "modelInvocation.embedding.maxBatchInputs", value: 8 }, { path: "modelInvocation.embedding.callTimeoutSeconds", value: 240 }]) });
    await page.screenshot({ animations: "disabled", path: testInfo.outputPath(`config-model-invocation-${locale}.png`) });
  });
}
