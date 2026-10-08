import { expect, test } from "@playwright/test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { createServer } from "node:net";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parse } from "yaml";
import { spawnManagedProcess, stopManagedProcess, type ManagedProcess } from "./support/managed-process";
import { routeActiveProfileApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";

let directory = "", api = "";
let backend: ManagedProcess | undefined;
let frontend: ResourceUiFrontend | undefined;
async function document() {
  const response = await fetch(`${api}/api/v1/system/config`, { headers: { Authorization: "Bearer profile-test-token" } });
  expect(response.ok).toBeTruthy();
  return (await response.json()).data;
}
test.beforeAll(async () => {
  test.setTimeout(240_000);
  directory = await mkdtemp(join(tmpdir(), "tron-context-config-browser-"));
  await writeFile(join(directory, "application.yaml"), "port: 8080\nvariableCrypto:\n  keyBase64: dGVzdC1zZWNyZXQ=\nshortTermMemory:\n  context:\n    familyContextMaxChars: 6100\n  compaction:\n    retainedHistoryRatio: '30%'\n", { mode: 0o600 });
  const port = await new Promise<number>((done, reject) => {
    const socket = createServer(); socket.on("error", reject); socket.listen(0, "127.0.0.1", () => {
      const address = socket.address(); if (!address || typeof address === "string") throw new Error("missing port");
      socket.close(() => done(address.port));
    });
  });
  api = `http://127.0.0.1:${port}`;
  let logs = "";
  backend = spawnManagedProcess("make", ["be-ui-acceptance"], {
    cwd: resolve(fileURLToPath(import.meta.url), "../../.."),
    env: { ...process.env, CARGO_NET_OFFLINE: "true", CARGO_PROFILE_DEV_DEBUG: "0", CARGO_INCREMENTAL: "0", TRON_ACCEPTANCE_DIRECTORY: directory, TRON_ACCEPTANCE_PORT: String(port) }, stdio: "pipe",
  });
  backend.stdout.on("data", chunk => logs = (logs + chunk.toString()).slice(-12000));
  backend.stderr.on("data", chunk => logs = (logs + chunk.toString()).slice(-12000));
  const deadline = Date.now() + 210_000;
  let ready = false;
  while (Date.now() < deadline) {
    if (backend.exitCode !== null) throw new Error(logs);
    try { ready = (await fetch(`${api}/api/v1/health`)).ok; } catch { /* owned startup */ }
    if (ready) break;
    await new Promise(done => setTimeout(done, 250));
  }
  if (!ready) throw new Error(logs);
  frontend = await startResourceUiFrontend("/settings/general", { VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_API_BASE_URL: "", TRON_FRONTEND_API_PROXY_TARGET: api });
});
test.afterAll(async () => {
  await stopResourceUiFrontend(frontend);
  await stopManagedProcess(backend);
  if (directory) await rm(directory, { recursive: true, force: true });
});
test("system configuration exposes omitted defaults and persists context order and strategies through real API/YAML", async ({ page }, testInfo) => {
  test.setTimeout(60_000);
  const filename = join(directory, "application.yaml");
  const original = await readFile(filename, "utf8");
  const initial = await document();
  const find = (d: {items: Array<{path: string; value: unknown}>}, path: string) => d.items.find(item => item.path === path)!.value;
  expect(find(initial, "shortTermMemory.context.providers.history.targetRatio")).toBe("30%");
  expect(JSON.stringify(initial)).not.toContain("dGVzdC1zZWNyZXQ=");
  expect(await readFile(filename, "utf8")).toBe(original);
  await seedResourceUiShell(page, "en-US");
  await routeActiveProfileApi(page);
  await page.goto(`${frontend!.baseUrl}/settings/general`);
  const expand = async () => {
    await page.locator('[aria-controls="system-config-groups"]').click();
    await page.locator('[aria-controls="system-config-group-shortTermMemory"]').click();
  };
  await expand();
  const order = page.getByRole("textbox", {name:"shortTermMemory.context.providers.order", exact:true});
  await expect(order).toHaveValue("seed_prompt_prefix\napplication_context\ncurrent_session_checkpoint\nsession_family_context\ncurrent_session_working_state\ncurrent_session_history");
  const entries = (await order.inputValue()).split("\n");
  [entries[0], entries[1]] = [entries[1], entries[0]];
  await order.fill(entries.join("\n"));
  for (const [field, value] of [["strategy", "semantic"], ["triggerRatio", "75%"], ["targetRatio", "25%"]]) {
    await page.getByRole("textbox", {name:`shortTermMemory.context.providers.working.${field}`, exact:true}).fill(value);
  }
  await page.getByRole("spinbutton", {name:"shortTermMemory.context.providers.maxAuxiliaryCallsPerStep", exact:true}).fill("2");
  const saved = page.waitForResponse(response => response.url().endsWith("/api/v1/system/config") && response.request().method() === "PUT");
  await page.getByRole("button", {name:/^Save/}).click();
  const response = await saved;
  expect(response.ok()).toBeTruthy();
  const result = (await response.json()).data;
  expect(result.hotUpdatePaths).toContain("shortTermMemory.context.providers.order");
  expect(result.restartPaths).toEqual([]);
  const yaml = parse(await readFile(filename, "utf8"));
  expect(yaml.shortTermMemory.context.providers.order).toEqual(entries);
  expect(yaml.shortTermMemory.context.providers.working.strategy).toBe("semantic");
  expect(yaml.shortTermMemory.context.familyContextMaxChars).toBe(6100);
  expect(yaml.shortTermMemory.context.providers.history).toBeUndefined();
  expect(find(await document(), "shortTermMemory.context.providers.history.targetRatio")).toBe("30%");
  await page.reload(); await expand();
  await expect(order).toHaveValue(entries.join("\n"));
  await expect(page.getByRole("textbox", {name:"shortTermMemory.context.providers.working.strategy", exact:true})).toHaveValue("semantic");
  await page.screenshot({path:testInfo.outputPath("context-config-saved.png"), fullPage:true});
  const beforeInvalid = await readFile(filename, "utf8");
  entries[1] = entries[0];
  await order.fill(entries.join("\n"));
  const rejected = page.waitForResponse(response => response.url().endsWith("/api/v1/system/config") && response.request().method() === "PUT");
  await page.getByRole("button", {name:/^Save/}).click();
  expect((await rejected).ok()).toBeFalsy();
  expect(await readFile(filename, "utf8")).toBe(beforeInvalid);
});
