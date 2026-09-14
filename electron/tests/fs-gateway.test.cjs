const test = require("node:test");
const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const { handleFsCommand, _internals } = require("../main/fs-gateway.cjs");

function makeRoot() {
  const realPath = fs.mkdtempSync(path.join(os.tmpdir(), "tron-fs-"));
  const rootId = crypto.randomUUID();
  _internals.roots.set(rootId, { realPath, name: path.basename(realPath) });
  return { rootId, realPath };
}

function cleanup(realPath) {
  fs.rmSync(realPath, { recursive: true, force: true });
}

test("isBinaryBuffer detects a NUL byte in the leading slice", () => {
  assert.equal(_internals.isBinaryBuffer(Buffer.from("abc\0def")), true);
  assert.equal(_internals.isBinaryBuffer(Buffer.from("plain text")), false);
});

test("isWithin rejects paths outside the root", () => {
  assert.equal(_internals.isWithin("/a/b", "/a/b/c"), true);
  assert.equal(_internals.isWithin("/a/b", "/a/b"), true);
  assert.equal(_internals.isWithin("/a/b", "/a/bc"), false);
  assert.equal(_internals.isWithin("/a/b", "/a"), false);
});

test("list returns directories first and skips unreadable entries", async () => {
  const { rootId, realPath } = makeRoot();
  fs.mkdirSync(path.join(realPath, "src"));
  fs.writeFileSync(path.join(realPath, "b.txt"), "b");
  fs.writeFileSync(path.join(realPath, "a.txt"), "a");

  const { entries } = await handleFsCommand("fs:list", { rootId, path: "" });
  assert.deepEqual(entries.map((entry) => entry.name), ["src", "a.txt", "b.txt"]);
  assert.equal(entries[0].isDir, true);

  cleanup(realPath);
});

test("read returns text content and flags binary content", async () => {
  const { rootId, realPath } = makeRoot();
  fs.writeFileSync(path.join(realPath, "a.txt"), "hello");
  fs.writeFileSync(path.join(realPath, "b.bin"), Buffer.from([0, 1, 2]));

  const text = await handleFsCommand("fs:read", { rootId, path: "a.txt" });
  assert.equal(text.content, "hello");
  assert.equal(text.binary, false);

  const binary = await handleFsCommand("fs:read", { rootId, path: "b.bin" });
  assert.equal(binary.binary, true);
  assert.equal(binary.content, "");

  cleanup(realPath);
});

test("path traversal is rejected", async () => {
  const { rootId, realPath } = makeRoot();
  await assert.rejects(
    () => handleFsCommand("fs:read", { rootId, path: "../secret" }),
    /path-outside-root/,
  );
  cleanup(realPath);
});

test("write round-trips through read", async () => {
  const { rootId, realPath } = makeRoot();
  await handleFsCommand("fs:write", { rootId, path: "new.txt", content: "data" });

  const result = await handleFsCommand("fs:read", { rootId, path: "new.txt" });
  assert.equal(result.content, "data");

  cleanup(realPath);
});

test("createFile and createDirectory reject duplicates", async () => {
  const { rootId, realPath } = makeRoot();

  await handleFsCommand("fs:createFile", { rootId, path: "f.txt" });
  await handleFsCommand("fs:createDirectory", { rootId, path: "d" });

  assert.equal(fs.existsSync(path.join(realPath, "f.txt")), true);
  assert.equal(fs.statSync(path.join(realPath, "d")).isDirectory(), true);

  await assert.rejects(
    () => handleFsCommand("fs:createFile", { rootId, path: "f.txt" }),
    /already-exists/,
  );

  cleanup(realPath);
});

test("rename and delete mutate the subtree", async () => {
  const { rootId, realPath } = makeRoot();
  await handleFsCommand("fs:write", { rootId, path: "a.txt", content: "x" });

  await handleFsCommand("fs:rename", { rootId, path: "a.txt", newName: "b.txt" });
  assert.equal(fs.existsSync(path.join(realPath, "b.txt")), true);

  await handleFsCommand("fs:delete", { rootId, path: "b.txt" });
  assert.equal(fs.existsSync(path.join(realPath, "b.txt")), false);

  cleanup(realPath);
});
