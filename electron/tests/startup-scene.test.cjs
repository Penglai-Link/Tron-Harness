const { test } = require("node:test");
const assert = require("node:assert/strict");
const { EventEmitter } = require("node:events");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { createStartupScene } = require("../main/startup-scene.cjs");
const { startupPageHtml } = require("../main/startup-page.cjs");

test("keeps the live startup view through navigation until the main animation is ready", async (t) => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), "tron-animation-"));
  t.after(() => fs.rmSync(dataDir, { recursive: true, force: true }));
  fs.writeFileSync(path.join(dataDir, "startup-theme.json"), '"light"');
  const window = new EventEmitter(), ipcMain = new EventEmitter();
  let attached, closed = false, background;
  window.webContents = {};
  window.isDestroyed = () => false;
  window.setBackgroundColor = (value) => { background = value; };
  window.getContentSize = () => [1280, 800];
  window.contentView = {
    addChildView(view) { attached = view; },
    removeChildView(view) { assert.equal(view, attached); attached = null; },
  };
  class View {
    constructor() { this.webContents = { loadURL: async () => {}, isDestroyed: () => false, close() { closed = true; } }; }
    setBounds(bounds) { this.bounds = bounds; }
    setBackgroundColor() {}
  }
  const scene = createStartupScene({ window, ipcMain, WebContentsView: View, dataDir });
  assert.equal(scene.theme, "light");
  assert.equal(background, "#ffffff");
  await scene.show("<html>loading</html>");
  assert.deepEqual(attached.bounds, { x: 0, y: 0, width: 1280, height: 800 });
  window.emit("did-finish-load");
  ipcMain.emit("tron-animation:ready", { sender: {} });
  assert.ok(attached, "page load and unrelated renderers cannot dismiss the scene");
  ipcMain.emit("tron-animation:ready", { sender: window.webContents });
  assert.equal(attached, null);
  assert.equal(closed, true);
  ipcMain.emit("tron-animation:theme", { sender: {} }, "matrix-dark");
  assert.equal(scene.theme, "light");
  ipcMain.emit("tron-animation:theme", { sender: window.webContents }, "matrix-dark");
  assert.equal(JSON.parse(fs.readFileSync(path.join(dataDir, "startup-theme.json"))), "matrix-dark");
  window.emit("closed");
  assert.equal(ipcMain.listenerCount("tron-animation:ready"), 0);
  assert.equal(ipcMain.listenerCount("tron-animation:theme"), 0);
});

test("startup markup applies the persisted theme before painting", () => {
  for (const theme of ["light", "matrix-dark"]) {
    const html = startupPageHtml({ message: "Starting", dataDir: "/tmp", logPath: "/tmp/log", theme });
    assert.ok(html.includes(`data-theme="${theme}"`));
    assert.ok(html.includes("uniform float lightTheme"));
  }
});

test("first startup stays dark until the page is ready, then remembers its theme", async (t) => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), "tron-animation-"));
  t.after(() => fs.rmSync(dataDir, { recursive: true, force: true }));
  const window = new EventEmitter(), ipcMain = new EventEmitter();
  let background, attached;
  window.webContents = {};
  window.isDestroyed = () => false;
  window.setBackgroundColor = (value) => { background = value; };
  window.getContentSize = () => [800, 600];
  window.contentView = { addChildView(view) { attached = view; }, removeChildView() { attached = null; } };
  class View {
    constructor() { this.webContents = { loadURL: async () => {}, isDestroyed: () => false, close() {} }; }
    setBounds() {}
    setBackgroundColor(value) { this.background = value; }
  }
  const scene = createStartupScene({ window, ipcMain, WebContentsView: View, dataDir });
  await scene.show("loading");
  ipcMain.emit("tron-animation:theme", { sender: window.webContents }, "light");
  assert.equal(scene.theme, "matrix-dark");
  assert.equal(background, "#020305");
  assert.equal(attached.background, "#020305");
  assert.equal(fs.existsSync(path.join(dataDir, "startup-theme.json")), false);
  ipcMain.emit("tron-animation:ready", { sender: window.webContents });
  assert.equal(attached, null);
  assert.equal(scene.theme, "light");
  assert.equal(JSON.parse(fs.readFileSync(path.join(dataDir, "startup-theme.json"))), "light");
  window.emit("closed");
  const nextWindow = new EventEmitter();
  nextWindow.webContents = {};
  nextWindow.setBackgroundColor = () => {};
  const nextScene = createStartupScene({ window: nextWindow, ipcMain, WebContentsView: View, dataDir });
  assert.equal(nextScene.theme, "light");
  nextWindow.emit("closed");
});

test("dismisses the startup view if the loaded page never reports readiness", async (t) => {
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), "tron-animation-"));
  t.after(() => fs.rmSync(dataDir, { recursive: true, force: true }));
  const window = new EventEmitter(), ipcMain = new EventEmitter();
  let attached = false;
  window.webContents = {};
  window.isDestroyed = () => false;
  window.setBackgroundColor = () => {};
  window.getContentSize = () => [800, 600];
  window.contentView = { addChildView() { attached = true; }, removeChildView() { attached = false; } };
  class View {
    constructor() { this.webContents = { loadURL: async () => {}, isDestroyed: () => false, close() {} }; }
    setBounds() {}
    setBackgroundColor() {}
  }
  const scene = createStartupScene({ window, ipcMain, WebContentsView: View, dataDir });
  await scene.show("loading");
  scene.dismissIfNotReadyAfter(10);
  await new Promise((resolve) => setTimeout(resolve, 30));
  assert.equal(attached, false);
  window.emit("closed");
});

test("preload dismisses startup only after the skipped-animation destination paints", (t) => {
  const original = global.MutationObserver;
  t.after(() => { global.MutationObserver = original; });
  let update, frame, destination = false;
  const sent = [];
  global.MutationObserver = class {
    constructor(callback) { update = callback; }
    observe() {}
    disconnect() {}
  };
  const window = {
    addEventListener(name, callback) { if (name === "DOMContentLoaded") callback(); },
    requestAnimationFrame(callback) { frame = callback; },
  };
  const document = {
    documentElement: { dataset: { theme: "light" } },
    querySelector(selector) { return destination && selector.includes("[data-tron-entry-ready]") ? {} : null; },
  };
  require("../preload/startup-animation.cjs")({ window, document, ipcRenderer: { send: (...args) => sent.push(args) } });
  assert.equal(sent.filter(([event]) => event === "tron-animation:ready").length, 0);
  destination = true;
  update();
  assert.equal(sent.filter(([event]) => event === "tron-animation:ready").length, 0);
  frame();
  update();
  assert.equal(sent.filter(([event]) => event === "tron-animation:ready").length, 1);
});

test("preload reveals a hidden mounted scene without waiting for an animation frame", (t) => {
  const original = global.MutationObserver;
  t.after(() => { global.MutationObserver = original; });
  global.MutationObserver = class { observe() {} disconnect() {} };
  const sent = [];
  const window = {
    addEventListener(name, callback) { if (name === "DOMContentLoaded") callback(); },
    requestAnimationFrame() { throw new Error("a hidden page cannot be expected to render a frame"); },
  };
  const document = {
    hidden: true,
    documentElement: { dataset: {} },
    querySelector(selector) { return selector === ".black-hole-scene" ? {} : null; },
  };
  require("../preload/startup-animation.cjs")({ window, document, ipcRenderer: { send: (...args) => sent.push(args) } });
  assert.deepEqual(sent, [["tron-animation:ready"]]);
});
