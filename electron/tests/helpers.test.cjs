const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const {
  buildGitHubOAuthRedirectUri,
  DEFAULT_PERSON_AI_SERVER_BASE_URL,
  iframeBlockReasonFromHeaders,
  isHttpUrl,
  resolveBackendConfigPath,
  resolveBackendLogPath,
  resolveControlDatabasePath,
  resolveControlDataDir,
  resolveProfileDataDir,
  resolveProfileDatabasePath,
  resolveProfileGraphStoragePath,
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

test("buildGitHubOAuthRedirectUri follows the desktop backend origin", () => {
  assert.equal(
    buildGitHubOAuthRedirectUri("http://127.0.0.1:8081/"),
    "http://127.0.0.1:8081/api/v1/auth/oauth2/callback/github"
  );
});

test("resolveBackendLogPath stores sidecar logs below the desktop data directory", () => {
  assert.equal(
    resolveBackendLogPath("/tmp/tron-data"),
    "/tmp/tron-data/logs/tron-backend.log"
  );
});

test("profile paths keep control metadata separate from profile data", () => {
  const root = "/tmp/tron-data";
  assert.equal(resolveControlDataDir(root), "/tmp/tron-data/control");
  assert.equal(resolveControlDatabasePath(root), "/tmp/tron-data/control/profiles.sqlite");
  assert.equal(resolveProfileDataDir(root, "profile-a"), "/tmp/tron-data/profiles/profile-a");
  assert.equal(resolveProfileDatabasePath(root, "profile-a"), "/tmp/tron-data/profiles/profile-a/tron.sqlite3");
  assert.equal(resolveProfileGraphStoragePath(root, "profile-a"), "/tmp/tron-data/profiles/profile-a/tron.graph.grafeo");
});

test("profile paths reject traversal and absolute path injection", () => {
  assert.throws(() => resolveProfileDataDir("/tmp/tron-data", "../other"), /invalid Tron profile id/);
  assert.throws(() => resolveProfileDataDir("/tmp/tron-data", "/tmp/other"), /invalid Tron profile id/);
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
