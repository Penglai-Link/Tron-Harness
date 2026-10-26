import { expect, test } from "@playwright/test";
import { mkdtemp, readFile, rm, writeFile, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createServer } from "node:net";
import { fileURLToPath } from "node:url";
import { spawnManagedProcess, stopManagedProcess, type ManagedProcess } from "./support/managed-process";
import { routeActiveProfileApi, seedResourceUiShell, startResourceUiFrontend, stopResourceUiFrontend, type ResourceUiFrontend } from "./support/resource-ui-harness";
let backend: ManagedProcess | undefined;
let frontend: ResourceUiFrontend | undefined;
let directory = "";
let backendLogs = "";
test.beforeAll(async () => {
  test.setTimeout(240_000);
  directory = await mkdtemp(join(tmpdir(), "tron-pty-browser-"));
  const port = await new Promise<number>((resolvePort, reject) => {
    const server = createServer(); server.on("error", reject); server.listen(0, "127.0.0.1", () => {
      const address = server.address(); if (!address || typeof address === "string") throw new Error("missing port");
      server.close(() => resolvePort(address.port));
    });
  });
  const root = resolve(fileURLToPath(import.meta.url), "../../..");
  backend = spawnManagedProcess("make", ["be-ui-acceptance", `CARGO=${process.env.TRON_ACCEPTANCE_CARGO || "cargo"}`], { cwd: root, env: { ...process.env,
    CARGO_PROFILE_DEV_DEBUG: process.env.CARGO_PROFILE_DEV_DEBUG || "0", CARGO_INCREMENTAL: process.env.CARGO_INCREMENTAL || "0", CARGO_BUILD_JOBS: "2",
    TRON_ACCEPTANCE_DIRECTORY: directory, TRON_ACCEPTANCE_PORT: String(port),
  }, stdio: "pipe" });
  backend.stdout.on("data", data => backendLogs += data.toString());
  backend.stderr.on("data", data => backendLogs += data.toString());
  const deadline = Date.now() + 210_000;
  let ready = false;
  while (Date.now() < deadline) {
    if (backend.exitCode !== null) throw new Error(backendLogs);
    try { ready = (await fetch(`http://127.0.0.1:${port}/api/v1/health`)).ok; } catch { /* compiling or starting */ }
    if (ready) break;
    await new Promise(resolveDelay => setTimeout(resolveDelay, 250));
  }
  if (!ready) throw new Error(`Acceptance backend did not start: ${backendLogs}`);
  frontend = await startResourceUiFrontend("/apps", { VITE_TRON_DATA_SOURCE: "mock", VITE_TRON_API_BASE_URL: "", TRON_FRONTEND_API_PROXY_TARGET: `http://127.0.0.1:${port}` });
});
test.afterAll(async () => {
  await stopResourceUiFrontend(frontend); await stopManagedProcess(backend);
  if (directory) await rm(directory, { recursive: true, force: true });
});
test("public agent-ui terminal executes through Tron's real PTY, resizes and closes", async ({ page }, info) => {
  test.setTimeout(30_000);
  await seedResourceUiShell(page);
  // Only identity bootstrap is a fixture; chat sessions, tickets, sockets and PTY are real.
  await routeActiveProfileApi(page);
  const frames: Array<{direction:string;payload:string}> = [];
  let promptEpoch = 0;
  const sizes: Array<{rows:number;cols:number}> = [];
  const ready = new Promise<void>(resolveReady => page.on("websocket", socket => {
    socket.on("framereceived", event => { frames.push({direction:"in",payload:String(event.payload)}); if (String(event.payload).includes(']133;B')) { promptEpoch += 1; resolveReady(); } });
    socket.on("framesent", event => {
      const payload = String(event.payload);
      frames.push({direction:"out",payload});
      if (!payload.startsWith("42")) return;
      try { const [name, size] = JSON.parse(payload.slice(payload.indexOf("["))); if (name === "terminal:resize") sizes.push(size); } catch { /* Socket.IO connection frame */ }
    });
  }));
  page.on("console", message => { if (message.type() === "error" || message.type() === "warning") console.log("PTY browser:", message.text()); });
  try {
  await page.goto(`${frontend!.baseUrl}/apps`);
  await page.getByRole("button", { name: "Open terminal", exact: true }).click();
  const input = page.locator(".xterm-helper-textarea");
  await expect(input).toBeAttached();
  await ready;
  const command = async (text: string) => {
    const previousPrompt = promptEpoch;
    await input.pressSequentially(text);
    await input.press("Enter");
    // A file can be written before the shell emits its next ready prompt.
    await expect.poll(() => promptEpoch).toBeGreaterThan(previousPrompt);
  };
  await input.focus();
  await command("printf '%s' 'real-pty-ok' > acceptance-result.txt");
  await expect.poll(() => readFile(join(directory, "acceptance-result.txt"), "utf8").catch(() => "")).toBe("real-pty-ok");
  await command("stty size > acceptance-initial-size.txt");
  await expect.poll(() => readFile(join(directory, "acceptance-initial-size.txt"), "utf8").catch(() => "")).toMatch(/^\s*\d+\s+\d+\s*$/);
  const [initialRows, initialCols] = (await readFile(join(directory, "acceptance-initial-size.txt"), "utf8")).trim().split(/\s+/).map(Number);
  await page.getByRole("button", { name: "Maximize terminal", exact: true }).click();
  await expect.poll(() => sizes.some(size => size.rows > initialRows && size.cols >= initialCols)).toBe(true);
  await input.focus();
  await command("stty size > acceptance-size.txt");
  await expect.poll(async () => {
    const [rows, cols] = (await readFile(join(directory, "acceptance-size.txt"), "utf8").catch(() => "0 0")).trim().split(/\s+/).map(Number);
    return rows > initialRows && cols >= initialCols;
  }).toBe(true);
  await input.pressSequentially("exit"); await input.press("Enter");
  await expect(input).toHaveCount(0);
  } finally {
    const path = info.outputPath("pty-evidence.json");
    await writeFile(path, JSON.stringify({frames,sizes,promptEpoch, files: await readdir(directory), screen: await page.locator("body").innerText()},null,2));
    await info.attach("pty-evidence", {path,contentType:"application/json"});
  }
});
