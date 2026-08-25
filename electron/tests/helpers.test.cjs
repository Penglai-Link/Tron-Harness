const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const {
  buildGitHubOAuthRedirectUri,
  iframeBlockReasonFromHeaders,
  isHttpUrl,
  resolveBackendConfigPath,
  resolveBackendLogPath,
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
