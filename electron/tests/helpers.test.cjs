const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const {
  DEFAULT_PERSON_AI_SERVER_BASE_URL,
  iframeBlockReasonFromHeaders,
  isHttpUrl,
  resolveBackendConfigPath,
  resolveBackendHost,
  resolveBackendLogPath,
  resolveControlDatabasePath,
  resolveControlDataDir,
  resolveDesktopDataDir,
  resolveSharedDatabasePath,
  resolveSharedGraphStoragePath,
  resolveUpdateServerUrl,
  sanitizeUrl
} = require("../main/helpers.cjs");

test("sanitizeUrl normalizes bare host input", () => {
  assert.equal(sanitizeUrl("example.com"), "https://example.com");
});

test("isHttpUrl accepts only http and https URLs", () => {
  assert.equal(isHttpUrl("https://example.com"), true);
  assert.equal(isHttpUrl("http://example.com"), true);
  assert.equal(isHttpUrl("file:///tmp/index.html"), false);
});

test("Windows packaged data follows the executable's custom installation directory", () => {
  assert.equal(resolveDesktopDataDir({ isPackaged: true, platformName: "win32", execPath: "E:\\应用 程序\\Tron\\Tron.exe" }), "E:\\应用 程序\\Tron\\data");
  assert.equal(resolveDesktopDataDir({ isPackaged: false, platformName: "win32", execPath: "E:\\Tron\\electron.exe", homeDir: os.homedir() }), path.join(os.homedir(), ".tron"));
  for (const platformName of ["darwin", "linux"]) {
    assert.equal(resolveDesktopDataDir({ isPackaged: true, platformName, homeDir: os.homedir() }), path.join(os.homedir(), ".tron"));
  }
});

test("install-local configuration keeps attachments and skills alongside the databases", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tron-local-config-"));
  const dataDir = path.join(root, "自定义 Tron", "data");
  const resourcesDir = path.join(root, "resources");
  try {
    fs.mkdirSync(dataDir, { recursive: true });
    fs.mkdirSync(resourcesDir);
    fs.copyFileSync(path.resolve(__dirname, "../../tron-backend/application.yaml"), path.join(resourcesDir, "application.yaml"));
    const configPath = resolveBackendConfigPath(dataDir, resourcesDir, { relocateStorage: true });
    const config = fs.readFileSync(configPath, "utf8");
    assert.ok(config.includes(`root: ${JSON.stringify(path.join(dataDir, "resources"))}`));
    assert.ok(config.includes(`skillsRegistryRoot: ${JSON.stringify(path.join(dataDir, "skills-registry"))}`));
    assert.ok(!config.includes("~/.tron"));
    fs.writeFileSync(configPath, "port: 9090\n");
    resolveBackendConfigPath(dataDir, resourcesDir, { relocateStorage: true });
    assert.equal(fs.readFileSync(configPath, "utf8"), "port: 9090\n", "existing user configuration must be preserved");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("resolveBackendLogPath stores sidecar logs below the desktop data directory", () => {
  assert.equal(
    resolveBackendLogPath("/tmp/tron-data"),
    path.join("/tmp/tron-data", "logs", "tron-backend.log")
  );
  assert.equal(
    resolveBackendLogPath("/tmp/tron-data/profiles/profile-a"),
    path.join("/tmp/tron-data/profiles/profile-a", "logs", "tron-backend.log")
  );
});

test("shared data paths keep control metadata separate from shared profile data", () => {
  const root = "/tmp/tron-data";
  assert.equal(resolveControlDataDir(root), path.join(root, "control"));
  assert.equal(resolveControlDatabasePath(root), path.join(root, "control", "profiles.sqlite"));
  assert.equal(resolveSharedDatabasePath(root), path.join(root, "tron.sqlite3"));
  assert.equal(resolveSharedGraphStoragePath(root), path.join(root, "tron.graph.grafeo"));
});

test("LAN binding is opt-in and never enabled for packaged builds", () => {
  const previous = process.env.TRON_ALLOW_LAN_DEV;
  process.env.TRON_ALLOW_LAN_DEV = "true";
  assert.equal(resolveBackendHost({ isPackaged: false }), "0.0.0.0");
  assert.equal(resolveBackendHost({ isPackaged: true }), "127.0.0.1");
  process.env.TRON_ALLOW_LAN_DEV = "false";
  assert.equal(resolveBackendHost({ isPackaged: false }), "127.0.0.1");
  if (previous === undefined) delete process.env.TRON_ALLOW_LAN_DEV;
  else process.env.TRON_ALLOW_LAN_DEV = previous;
});

test("resolveUpdateServerUrl prefers runtime environment over packaged config", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tron-update-config-"));
  fs.writeFileSync(path.join(root, "update-config.json"), JSON.stringify({ versionServerUrl: "https://packaged.example.com" }));
  const previous = process.env.TRON_VERSION_SERVER_URL;

  delete process.env.TRON_VERSION_SERVER_URL;
  assert.equal(resolveUpdateServerUrl(root), "https://packaged.example.com");
  process.env.TRON_VERSION_SERVER_URL = "https://runtime.example.com";
  assert.equal(resolveUpdateServerUrl(root), "https://runtime.example.com");

  if (previous === undefined) delete process.env.TRON_VERSION_SERVER_URL;
  else process.env.TRON_VERSION_SERVER_URL = previous;
  fs.rmSync(root, { recursive: true, force: true });
});

test("resolveUpdateServerUrl falls back to the Person-AI SDK server", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tron-update-default-"));
  const previous = process.env.TRON_VERSION_SERVER_URL;

  delete process.env.TRON_VERSION_SERVER_URL;
  assert.equal(resolveUpdateServerUrl(root), DEFAULT_PERSON_AI_SERVER_BASE_URL);

  if (previous === undefined) delete process.env.TRON_VERSION_SERVER_URL;
  else process.env.TRON_VERSION_SERVER_URL = previous;
  fs.rmSync(root, { recursive: true, force: true });
});

test("resolveBackendConfigPath initializes user config without overwriting it", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tron-config-"));
  const dataDir = path.join(root, "data");
  const resourcesDir = path.join(root, "resources");
  fs.mkdirSync(dataDir);
  fs.mkdirSync(resourcesDir);
  fs.writeFileSync(path.join(resourcesDir, "application.yaml"), "port: 8080\n");

  const configPath = resolveBackendConfigPath(dataDir, resourcesDir);
  assert.equal(configPath, path.join(dataDir, "application.yaml"));
  assert.equal(fs.readFileSync(configPath, "utf8"), "port: 8080\n");

  fs.writeFileSync(configPath, "port: 9090\n");
  assert.equal(resolveBackendConfigPath(dataDir, resourcesDir), configPath);
  assert.equal(fs.readFileSync(configPath, "utf8"), "port: 9090\n");

  fs.rmSync(root, { recursive: true, force: true });
});

test("resolveBackendConfigPath clears the bundled crypto key for a new profile", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tron-config-profile-"));
  const dataDir = path.join(root, "profile");
  const resourcesDir = path.join(root, "resources");
  fs.mkdirSync(dataDir, { recursive: true });
  fs.mkdirSync(resourcesDir, { recursive: true });
  fs.writeFileSync(path.join(resourcesDir, "application.yaml"), "variableCrypto:\n  keyBase64: bundled-key\n");

  const configPath = resolveBackendConfigPath(dataDir, resourcesDir, { profileConfig: true });

  assert.match(fs.readFileSync(configPath, "utf8"), /keyBase64: \"\"/);
});

test("iframeBlockReasonFromHeaders detects x-frame-options deny", () => {
  assert.equal(
    iframeBlockReasonFromHeaders({ "x-frame-options": "DENY" }),
    "x-frame-options-deny"
  );
});

test("iframeBlockReasonFromHeaders detects frame ancestors none", () => {
  assert.equal(
    iframeBlockReasonFromHeaders({
      "content-security-policy": "default-src 'self'; frame-ancestors 'none'"
    }),
    "csp-frame-ancestors-none"
  );
});

test("iframeBlockReasonFromHeaders allows unrestricted headers", () => {
  assert.equal(iframeBlockReasonFromHeaders({}), null);
});
