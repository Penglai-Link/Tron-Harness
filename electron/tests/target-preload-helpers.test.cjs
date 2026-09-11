const test = require("node:test");
const assert = require("node:assert/strict");

const {
  buildPageMessageScript,
  normalizeTargetConfiguration
} = require("../preload/target-preload-helpers.cjs");

test("page messages are dispatched in the page main world", () => {
  assert.equal(
    buildPageMessageScript({ channel: "tron:frame-event", type: "tron:toggle-injection" }),
    'window.postMessage({"channel":"tron:frame-event","type":"tron:toggle-injection"}, "*");'
  );
});

test("target configuration preserves the comment capability", () => {
  assert.deepEqual(
    normalizeTargetConfiguration({
      commentAvailable: true,
      enabled: false,
      pageId: " page-1 "
    }),
    {
      commentAvailable: true,
      enabled: false,
      pageId: "page-1"
    }
  );
});

test("target configuration requires a valid page id", () => {
  assert.equal(normalizeTargetConfiguration({ commentAvailable: true }), null);
  assert.equal(normalizeTargetConfiguration({ pageId: "   " }), null);
});
