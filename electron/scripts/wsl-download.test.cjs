const assert = require("node:assert/strict");
const { EventEmitter } = require("node:events");
const { Readable, PassThrough } = require("node:stream");
const { test } = require("node:test");
const { fetchWslDownload } = require("../main/wsl-download.cjs");

function fixture({ redirects = [], body = Readable.from([Buffer.from("backend")]), pending = false } = {}) {
  const request = new EventEmitter();
  let followed = 0, aborted = false, created = 0;
  const next = () => {
    if (aborted || pending) return;
    const location = redirects.shift();
    if (location) request.emit("redirect", 302, "GET", location, {});
    else {
      body.statusCode = 200;
      body.headers = { "content-length": "7", "content-type": "application/octet-stream" };
      body.once("close", () => request.emit("close"));
      request.emit("response", body);
    }
  };
  request.end = () => queueMicrotask(next);
  request.followRedirect = () => { followed++; queueMicrotask(next); };
  request.abort = () => {
    if (aborted) return;
    aborted = true;
    request.emit("abort");
    request.emit("close");
  };
  const net = { request: (options) => {
    created++;
    assert.equal(options.redirect, "manual");
    return request;
  } };
  return { net, request, get followed() { return followed; }, get aborted() { return aborted; }, get created() { return created; } };
}

test("Electron streaming download records the actual HTTPS URL without Response.url from net.fetch", async () => {
  const f = fixture();
  const response = await fetchWslDownload(f.net, "https://downloads.example/backend");
  assert.equal(response.url, "https://downloads.example/backend");
  assert.equal(response.status, 200);
  assert.equal(response.redirected, false);
  assert.equal(response.headers.get("content-length"), "7");
  assert.equal(await response.text(), "backend");
});

test("HTTPS CDN redirects are checked and followed before streaming", async () => {
  const f = fixture({ redirects: ["https://cdn.example/releases/backend", "backend?token=test"] });
  const response = await fetchWslDownload(f.net, "https://downloads.example/backend");
  assert.equal(response.url, "https://cdn.example/releases/backend?token=test");
  assert.equal(response.redirected, true);
  assert.equal(f.followed, 2);
  assert.equal(await response.text(), "backend");
});

for (const url of ["http://downloads.example/backend", "file:///backend", "https://user:secret@downloads.example/backend"]) {
  test(`rejects insecure initial download: ${new URL(url).protocol}`, async () => {
    const f = fixture();
    await assert.rejects(fetchWslDownload(f.net, url), /wsl-insecure-download-url/);
    assert.equal(f.created, 0);
  });
}

for (const redirect of ["http://downloads.example/backend", "http://127.0.0.1/backend", "file:///backend"]) {
  test(`blocks HTTPS downgrade before following: ${new URL(redirect).protocol}`, async () => {
    const f = fixture({ redirects: [redirect, "https://cdn.example/backend"] });
    await assert.rejects(fetchWslDownload(f.net, "https://downloads.example/backend"), /wsl-insecure-download-url/);
    assert.equal(f.followed, 0);
    assert.equal(f.aborted, true);
  });
}

test("keeps local HTTP development downloads working", async () => {
  const f = fixture({ redirects: ["/backend"] });
  const response = await fetchWslDownload(f.net, "http://127.0.0.1:8080/release");
  assert.equal(response.url, "http://127.0.0.1:8080/backend");
  assert.equal(await response.text(), "backend");
});

test("limits redirect loops", async () => {
  const f = fixture({ redirects: Array(11).fill("https://cdn.example/backend") });
  await assert.rejects(fetchWslDownload(f.net, "https://downloads.example/backend"), /too-many-redirects/);
  assert.equal(f.followed, 10);
  assert.equal(f.aborted, true);
});

test("already aborted download does not start a request", async () => {
  const f = fixture();
  await assert.rejects(fetchWslDownload(f.net, "https://downloads.example/backend", { signal: AbortSignal.abort() }), { name: "AbortError" });
  assert.equal(f.created, 0);
});

test("abort before headers cancels the Chromium request", async () => {
  const f = fixture({ pending: true });
  const controller = new AbortController();
  const result = fetchWslDownload(f.net, "https://downloads.example/backend", { signal: controller.signal });
  controller.abort();
  await assert.rejects(result, { name: "AbortError" });
  assert.equal(f.aborted, true);
});

test("timeout during a download fails the body reader and cancels the request", async () => {
  const f = fixture({ body: new PassThrough() });
  const controller = new AbortController();
  const response = await fetchWslDownload(f.net, "https://downloads.example/backend", { signal: controller.signal });
  const result = response.text();
  controller.abort();
  await assert.rejects(result, { name: "AbortError" });
  assert.equal(f.aborted, true);
});

test("cancelling a response stops its Chromium download", async () => {
  const f = fixture({ body: new PassThrough() });
  const response = await fetchWslDownload(f.net, "https://downloads.example/backend");
  await response.body.cancel();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(f.aborted, true);
});
