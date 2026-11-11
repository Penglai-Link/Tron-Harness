const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { EventEmitter } = require("node:events");
const { PassThrough } = require("node:stream");
const { spawn } = require("node:child_process");
const { once } = require("node:events");
const { setTimeout: delay } = require("node:timers/promises");
const { test } = require("node:test");
const { Updater, UPDATE_HELPER_READY, currentOsType, runApplyUpdate, waitForLauncherExit } = require("../main/updater.cjs");

function directory(t) {
  const parent = path.resolve(os.tmpdir());
  const root = fs.mkdtempSync(path.join(parent, "tron-updater-test-"));
  t.after(() => {
    assert.equal(path.dirname(path.resolve(root)), parent);
    assert.ok(path.basename(root).startsWith("tron-updater-test-"));
    fs.rmSync(root, { recursive: true, force: true });
  });
  return root;
}

function fakeChild() {
  const child = new EventEmitter();
  child.stdin = new PassThrough();
  child.stdout = new PassThrough();
  child.stderr = new PassThrough();
  child.unref = () => {};
  child.kill = () => { child.killed = true; };
  return child;
}

function fixture(t, platformName = "win32", overrides = {}) {
  const root = directory(t);
  const calls = [], opened = [], exits = [], statuses = [];
  const name = overrides.fileName ?? (platformName === "win32" ? "Tron Setup.exe" : platformName === "darwin" ? "Tron.dmg" : "Tron.AppImage");
  const updater = new Updater({
    dataDir: root, platformName, archName: "x64", getVersion: () => "1.0.1",
    execPath: path.join(root, "Tron.exe"), getVersionServerUrl: () => "https://updates.example",
    fetchImpl: async (url) => {
      url = String(url);
      if (url.includes("/versions/latest")) return Response.json({ code: "SUCCESS", data: { id: 2, version_number: "1.0.2", packages: [{ os_type: currentOsType(platformName, "x64"), file_name: name }] } });
      if (url.includes("/download?")) return Response.json({ code: "SUCCESS", data: { download_url: "https://downloads.example/package" } });
      return new Response(Buffer.from("update"), { headers: { "Content-Length": String(overrides.total ?? 6) } });
    },
    spawnImpl: (...args) => { const child = fakeChild(); calls.push({ args, child }); queueMicrotask(() => child.stdout.write(`${UPDATE_HELPER_READY}\n`)); return child; },
    openInstaller: async (file) => { opened.push(file); return ""; },
    exit: (code) => exits.push(code), onStatus: (status) => statuses.push(status),
    ...overrides,
  });
  return { updater, root, calls, opened, exits, statuses, name };
}

test("Windows downloads, waits for helper readiness, and schedules one restart", async (t) => {
  let signalSpawn;
  const spawned = new Promise((resolve) => { signalSpawn = resolve; });
  const child = fakeChild();
  const f = fixture(t, "win32", { spawnImpl: (...args) => { f.calls.push({ args, child }); signalSpawn(); return child; } });
  await f.updater.check("test-token");
  const download = f.updater.startDownload("test-token");
  await f.updater.startDownload("test-token");
  await spawned;
  assert.equal(f.calls.length, 1);
  assert.equal(f.updater.getStatus().type, "installing");
  assert.equal((await f.updater.check("test-token")).type, "installing");
  await f.updater.apply();
  assert.deepEqual(f.exits, []);
  assert.equal(f.calls[0].args[1][0], "--_tron-apply-update");
  assert.equal(f.calls[0].args[1].at(-2), f.root);
  assert.equal(f.calls[0].args[1].at(-1), String(process.pid));
  assert.equal(f.calls[0].args[2].cwd, os.tmpdir());
  child.stdout.write(UPDATE_HELPER_READY.slice(0, 8));
  child.stdout.write(`${UPDATE_HELPER_READY.slice(8)}\n`);
  await download;
  await delay(650);
  assert.deepEqual(f.exits, [0]);
  assert.deepEqual(f.statuses.map((status) => status.type).filter((type, i, all) => type !== all[i - 1]), ["checking", "updateAvailable", "downloading", "downloaded", "installing"]);
});

for (const failure of ["spawn", "exit"]) {
  test(`Windows helper ${failure} failure keeps the current app running`, async (t) => {
    const f = fixture(t, "win32", { spawnImpl: () => {
      const child = fakeChild();
      queueMicrotask(() => failure === "spawn" ? child.emit("error", new Error("helper launch denied")) : child.emit("exit", 1));
      return child;
    } });
    await f.updater.check("test-token");
    await assert.rejects(f.updater.startDownload("test-token"), failure === "spawn" ? /launch denied/ : /exited before ready/);
    assert.equal(f.updater.getStatus().type, "error");
    await delay(650);
    assert.deepEqual(f.exits, []);
  });
}

for (const total of [0, 20]) {
  test(`an ${total ? "incomplete" : "empty"} download never launches an installer`, async (t) => {
    const f = fixture(t, "win32", total ? { total } : { fetchImpl: async (url) => {
      url = String(url);
      if (url.includes("/versions/latest")) return Response.json({ id: 2, version_number: "1.0.2", packages: [{ os_type: 120 }] });
      if (url.includes("/download?")) return Response.json({ download_url: "https://downloads.example/package" });
      return new Response(new Uint8Array());
    } });
    await f.updater.check("test-token");
    await assert.rejects(f.updater.startDownload("test-token"), /incomplete/);
    assert.equal(f.updater.getStatus().type, "error");
    assert.deepEqual(f.calls, []);
    assert.deepEqual(f.exits, []);
    assert.deepEqual(fs.readdirSync(path.join(f.root, "updates")), []);
  });
}

for (const [platform, fileName] of [["darwin", "Tron.dmg"], ["linux", "Tron.AppImage"], ["linux", "Tron.deb"], ["linux", "Tron.rpm"]]) {
  test(`${platform} ${fileName} opens the manual update without quitting`, async (t) => {
    const f = fixture(t, platform, { fileName });
    assert.equal((await f.updater.check("test-token")).installationMode, "manual");
    await f.updater.startDownload("test-token");
    const status = f.updater.getStatus();
    assert.equal(status.type, "downloaded");
    assert.equal(status.fileName, fileName);
    assert.equal(status.installationMode, "manual");
    assert.deepEqual(f.opened, [status.filePath]);
    assert.deepEqual(f.calls, []);
    assert.deepEqual(f.exits, []);
    if (fileName.endsWith(".AppImage") && process.platform !== "win32") assert.equal(fs.statSync(status.filePath).mode & 0o777, 0o755);
    await f.updater.apply();
    assert.equal(f.opened.length, 2);
  });
}

test("failure to open a manual installer is reported without exiting", async (t) => {
  const f = fixture(t, "darwin", { openInstaller: async () => "Cannot open disk image" });
  await f.updater.check("test-token");
  await assert.rejects(f.updater.startDownload("test-token"), /Cannot open disk image/);
  assert.equal(f.updater.getStatus().type, "error");
  assert.deepEqual(f.exits, []);
});

test("Windows helper waits for launcher exit and passes the NSIS restart flags", async (t) => {
  const root = directory(t), input = new PassThrough(), calls = [];
  let signalReady;
  const ready = new Promise((resolve) => { signalReady = resolve; });
  const result = runApplyUpdate(path.join(root, "setup.exe"), {
    platformName: "win32", input, installDir: "C:\\Apps With Spaces\\Tron",
    logPath: path.join(root, "update.log"), onReady: signalReady,
    spawnImpl: (...args) => { calls.push(args); const child = fakeChild(); queueMicrotask(() => child.emit("spawn")); return child; },
  });
  await ready;
  assert.equal(calls.length, 0);
  input.end();
  assert.equal(await result, 0);
  assert.deepEqual(calls[0][1], ["--updated", "/S", "--force-run", "/D=C:\\Apps With Spaces\\Tron"]);
  assert.equal(calls[0][2].cwd, os.tmpdir());
});

test("installer spawn errors are logged rather than reported as success", async (t) => {
  const root = directory(t), input = new PassThrough();
  input.destroy();
  const logPath = path.join(root, "update.log");
  const result = await runApplyUpdate(path.join(root, "setup.exe"), {
    platformName: "win32", input, logPath,
    spawnImpl: () => { const child = fakeChild(); queueMicrotask(() => child.emit("error", new Error("installer missing"))); return child; },
  });
  assert.equal(result, 1);
  assert.match(fs.readFileSync(logPath, "utf8"), /update helper failed: installer missing/);
});

test("helper does not wait forever when the launcher pipe already closed", async () => {
  const input = new PassThrough();
  input.destroy();
  await waitForLauncherExit(input);
});

test("a closed Electron stdin cannot start installation while the launcher PID is alive", async (t) => {
  const parent = spawn(process.execPath, ["-e", "setInterval(() => {}, 1000)"], { stdio: "ignore", windowsHide: true });
  t.after(() => { if (parent.exitCode === null) parent.kill(); });
  await once(parent, "spawn");
  const closed = once(parent, "close");
  const input = new PassThrough();
  input.destroy();
  let exited = false;
  const waiting = waitForLauncherExit(input, parent.pid).then(() => { exited = true; });
  await delay(150);
  assert.equal(exited, false);
  parent.kill();
  await closed;
  await waiting;
  assert.equal(exited, true);
});
