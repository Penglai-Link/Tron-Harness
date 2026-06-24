const test = require("node:test");
const assert = require("node:assert/strict");

const {
  iframeBlockReasonFromHeaders,
  isHttpUrl,
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
