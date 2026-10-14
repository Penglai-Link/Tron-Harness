const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const fsp = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");

const {
  OS_TYPES,
  Updater,
  compareVersions,
  currentOsType,
  runApplyUpdate
} = require("../main/updater.cjs");

function jsonResponse(body, status = 200) {
  return new Response(JSON.stringify(body), {
    headers: { "content-type": "application/json" },
    status
  });
}

function makeUpdater(options = {}) {
  return new Updater({
    archName: "x64",
    dataDir: options.dataDir,
    fetchImpl: options.fetchImpl,
    getVersion: () => "1.2.0",
    getVersionServerUrl: () => "https://updates.example.com",
    onStatus: options.onStatus,
    platformName: options.platformName ?? "win32",
    ...options
  });
}

test("compares relaxed semantic versions", () => {
  assert.equal(compareVersions("v1.2.0", "1.2"), 0);
  assert.equal(compareVersions("1.10.0", "1.9.9"), 1);
  assert.equal(compareVersions("1.2.0-beta", "1.2.0"), 0);
  assert.equal(compareVersions("0.9.9", "1.0.0"), -1);
});

test("maps Windows and macOS architectures to version service os_type", () => {
  assert.equal(currentOsType("win32", "x64"), OS_TYPES.WINDOWS_X64);
  assert.equal(currentOsType("win32", "arm64"), OS_TYPES.WINDOWS_ARM64);
  assert.equal(currentOsType("darwin", "x64"), OS_TYPES.MACOS_X64);
  assert.equal(currentOsType("darwin", "arm64"), OS_TYPES.MACOS_ARM64);
});

test("returns upToDate when version service has no release", async () => {
  const updater = makeUpdater({ fetchImpl: async () => jsonResponse({ message: "not found" }, 404) });
  const status = await updater.check("token-1");
  assert.deepEqual(status, { type: "upToDate" });
});

test("rejects a latest response without a version number", async () => {
  const updater = makeUpdater({
    fetchImpl: async () => jsonResponse({ id: "version-2", packages: [] })
  });

  await assert.rejects(() => updater.check("token-1"), /no valid version number/);
  assert.equal(updater.getStatus().type, "error");
});

test("rejects HTTP errors and invalid download or install transitions", async () => {
  const updater = makeUpdater({
    fetchImpl: async () => jsonResponse({ message: "service unavailable" }, 503)
  });

  await assert.rejects(() => updater.check("token-1"), /service unavailable/);
  await assert.rejects(() => updater.startDownload("token-1"), /no downloadable update is available/);
  await assert.rejects(() => updater.apply(), /no downloaded update is available/);
});

test("finds a platform-specific update and sends product authentication headers", async () => {
  const requests = [];
  const updater = makeUpdater({
    fetchImpl: async (url, init) => {
      requests.push({ init, url: String(url) });
      return jsonResponse({
        code: 200,
        data: {
          id: "version-2",
          packages: [{ file_name: "Tron_2.0.0.exe", os_type: OS_TYPES.WINDOWS_X64 }],
          release_date: "2026-09-20",
          version_number: "v2.0.0"
        }
      });
    }
  });

  const status = await updater.check("token-1");
  assert.deepEqual(status, {
    fileName: "Tron_2.0.0.exe",
    releaseDate: "2026-09-20",
    type: "updateAvailable",
    version: "2.0.0",
    versionId: "version-2"
  });
  assert.match(requests[0].url, /\/api\/v1\/versions\/latest\?product_code=tron$/);
  assert.equal(requests[0].init.headers.Authorization, "Bearer token-1");
  assert.equal(requests[0].init.headers["X-Product-Code"], "tron");
});

test("treats missing current-platform packages as no applicable update", async () => {
  for (const packages of [[], [{ os_type: OS_TYPES.MACOS_ARM64 }]]) {
    const logs = [];
    const updater = makeUpdater({ onLog: (...entry) => logs.push(entry), fetchImpl: async () => jsonResponse({
      id: "version-2", version_number: "2.0.0", packages
    }) });
    updater.update = { version: "stale" };
    assert.deepEqual(await updater.check("token-1"), { type: "upToDate" });
    assert.equal(updater.update, null);
    assert.ok(logs.some(([level, message]) => level === "INFO" && message.includes("No update package")));
    assert.ok(logs.every(([level]) => level !== "ERROR"));
  }
});

test("downloads the signed package with streaming progress and removes partial files", async () => {
  const root = await fsp.mkdtemp(path.join(os.tmpdir(), "tron-updater-"));
  const calls = [];
  const updater = makeUpdater({
    dataDir: root,
    fetchImpl: async (url, init) => {
      calls.push({ init, url: String(url) });
      if (String(url).includes("/versions/latest")) {
        return jsonResponse({
          id: "version-2",
          packages: [{ file_name: "Tron_2.0.0.exe", os_type: OS_TYPES.WINDOWS_X64 }],
          version_number: "2.0.0"
        });
      }
      if (String(url).includes("/versions/version-2/download")) {
        return jsonResponse({ download_url: "https://cdn.example.com/Tron_2.0.0.exe" });
      }
      return new Response(Buffer.from("installer-bytes"), {
        headers: { "content-length": "15" },
        status: 200
      });
    }
  });

  await updater.check("token-1");
  await updater.startDownload("token-1");

  const status = updater.getStatus();
  assert.equal(status.type, "downloaded");
  assert.equal(fs.readFileSync(status.filePath, "utf8"), "installer-bytes");
  assert.equal(fs.existsSync(`${status.filePath}.part`), false);
  assert.equal(calls[1].init.headers.Authorization, "Bearer token-1");
  assert.equal(calls[2].init.headers?.Authorization, undefined);

  await fsp.rm(root, { recursive: true, force: true });
});

test("removes a partial package after the download stream is interrupted", async () => {
  const root = await fsp.mkdtemp(path.join(os.tmpdir(), "tron-updater-interrupted-"));
  const updater = makeUpdater({
    dataDir: root,
    fetchImpl: async (url) => {
      if (String(url).includes("/versions/latest")) {
        return jsonResponse({
          id: "version-2",
          packages: [{ file_name: "Tron_2.0.0.exe", os_type: OS_TYPES.WINDOWS_X64 }],
          version_number: "2.0.0"
        });
      }
      if (String(url).includes("/versions/version-2/download")) {
        return jsonResponse({ download_url: "https://cdn.example.com/Tron_2.0.0.exe" });
      }

      const body = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode("partial"));
          controller.error(new Error("network interrupted"));
        }
      });
      return new Response(body, { status: 200 });
    }
  });

  await updater.check("token-1");
  await assert.rejects(() => updater.startDownload("token-1"), /network interrupted/);
  assert.equal(updater.getStatus().type, "error");
  assert.equal(fs.existsSync(path.join(root, "updates", "Tron_2.0.0.exe.part")), false);

  await fsp.rm(root, { recursive: true, force: true });
});

test("Windows apply starts an isolated helper and waits before launching NSIS", async () => {
  const root = await fsp.mkdtemp(path.join(os.tmpdir(), "tron-updater-apply-"));
  const installerPath = path.join(root, "Tron_2.0.0.exe");
  await fsp.writeFile(installerPath, "installer");
  const calls = [];
  const updater = makeUpdater({
    dataDir: root,
    spawnImpl: (...args) => {
      calls.push(args);
      return { stdin: { unref() {} }, unref() {} };
    }
  });
  updater.status = { filePath: installerPath, type: "downloaded", version: "2.0.0" };
  await updater.apply();

  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0][1], ["--_tron-apply-update", installerPath, path.join(root, "logs", "tron-updater.log")]);
  assert.equal(updater.getStatus().type, "installing");

  await fsp.rm(root, { recursive: true, force: true });
});

test("macOS apply opens the DMG and exits after handing off to Finder", async () => {
  const root = await fsp.mkdtemp(path.join(os.tmpdir(), "tron-updater-macos-"));
  const installerPath = path.join(root, "Tron_2.0.0.dmg");
  await fsp.writeFile(installerPath, "installer");
  const opened = [];
  let exitCode;
  const updater = makeUpdater({
    dataDir: root,
    platformName: "darwin",
    openInstaller: async (filePath) => {
      opened.push(filePath);
      return "";
    },
    exit: (code) => {
      exitCode = code;
    }
  });
  updater.status = { filePath: installerPath, type: "downloaded", version: "2.0.0" };

  await updater.apply();

  assert.deepEqual(opened, [installerPath]);
  assert.equal(updater.getStatus().type, "installing");
  await new Promise((resolve) => setTimeout(resolve, 650));
  assert.equal(exitCode, 0);

  await fsp.rm(root, { recursive: true, force: true });
});

test("reports installation failures as an error state", async () => {
  const root = await fsp.mkdtemp(path.join(os.tmpdir(), "tron-updater-install-error-"));
  const installerPath = path.join(root, "Tron_2.0.0.dmg");
  await fsp.writeFile(installerPath, "installer");
  const updater = makeUpdater({
    dataDir: root,
    platformName: "darwin",
    openInstaller: async () => "Finder could not open the DMG"
  });
  updater.status = { filePath: installerPath, type: "downloaded", version: "2.0.0" };

  await assert.rejects(() => updater.apply(), /Finder could not open the DMG/);
  assert.deepEqual(updater.getStatus(), {
    message: "Finder could not open the DMG",
    type: "error",
    version: "2.0.0"
  });

  await fsp.rm(root, { recursive: true, force: true });
});

test("Windows helper launches silent installer after launcher stdin closes", async () => {
  const input = {
    once(event, callback) {
      if (event === "end") setImmediate(callback);
      return this;
    },
    resume() {}
  };
  const calls = [];
  const result = runApplyUpdate("C:\\Users\\user\\Tron_2.0.0.exe", {
    input,
    logPath: path.join(os.tmpdir(), `tron-updater-test-${Date.now()}.log`),
    platformName: "win32",
    spawnImpl: (...args) => {
      calls.push(args);
      return { unref() {} };
    }
  });
  assert.equal(await result, 0);
  assert.deepEqual(calls[0][1], ["/S", "/R"]);
});


test("persists real update failures as searchable application JSON logs", async (t) => {
  const directory = await fsp.mkdtemp(path.join(os.tmpdir(), "tron-update-logs-"));
  t.after(() => fsp.rm(directory, { recursive: true, force: true }));
  const { createApplicationLogger } = require("../main/application-log.cjs");
  const updater = makeUpdater({ onLog: createApplicationLogger(directory), fetchImpl: async () => jsonResponse({ message: "service unavailable" }, 503) });
  await assert.rejects(() => updater.check("private-access-token"));
  const files = await fsp.readdir(directory);
  assert.equal(files.length, 1);
  assert.match(files[0], /^tron-backend\.\d{4}-\d{2}-\d{2}\.jsonl$/);
  const contents = await fsp.readFile(path.join(directory, files[0]), "utf8");
  const records = contents.trim().split("\n").map(JSON.parse);
  assert.ok(records.some(record => record.level === "ERROR" && record.target === "tron.desktop.updater" && record.message.includes("service unavailable")));
  assert.ok(!contents.includes("private-access-token"));
});
