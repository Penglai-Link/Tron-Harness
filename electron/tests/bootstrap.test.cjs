const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const vm = require("node:vm");
const { EventEmitter } = require("node:events");
const { PassThrough } = require("node:stream");
const { finished } = require("node:stream/promises");

// Run the actual bootstrap with desktop/sidecar boundaries replaced. No GUI,
// persistent product files, network request or real subprocess is created.
test("bootstrap passes the desktop data directory to sidecar config, logs and cwd", async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tron-bootstrap-"));
  const dataDir = path.join(root, "data");
  const resourcesDir = path.join(root, "resources");
  fs.mkdirSync(path.join(resourcesDir, "binaries"), { recursive: true });
  fs.writeFileSync(path.join(resourcesDir, "binaries", process.platform === "win32" ? "tron-backend.exe" : "tron-backend"), "fixture only");
  const spawnCalls = [], logCalls = [], streams = [], windows = [];
  const handlers = new Map();
  const mainDir = path.resolve(__dirname, "../main");
  const helpers = require("../main/helpers.cjs");
  let clock = 0;
  class FastClock extends Date { static now() { return clock += 2000; } }
  class FakeWindow extends EventEmitter {
    constructor() {
      super(); windows.push(this); this.urls = [];
      this.webContents = new EventEmitter();
      this.webContents.setWindowOpenHandler = () => {};
      this.webContents.openDevTools = () => {};
    }
    async loadURL(url) { this.urls.push(url); }
    show() { this.shown = true; }
    focus() { this.focused = true; }
  }
  const child = new EventEmitter();
  child.stdout = new PassThrough(); child.stderr = new PassThrough();
  child.kill = () => { child.emit("close", 0, null); return true; };
  const context = vm.createContext({
    console, setTimeout, clearTimeout, Date: FastClock, __dirname: mainDir,
    process: { argv: [], env: {}, platform: process.platform },
    require(name) {
      if (name === "electron") return {
        app: { isPackaged: false, whenReady: () => Promise.resolve(), on: (name, cb) => handlers.set(name, cb) },
        BrowserWindow: FakeWindow, ipcMain: { handle() {} }, clipboard: {}, Menu: {}, shell: {}
      };
      if (name === "node:fs") return { ...fs,
        mkdirSync: (filename, options) => fs.mkdirSync(path.isAbsolute(filename) ? filename : path.join(root, filename), options),
        createWriteStream(filename, options) {
          logCalls.push(filename);
          const stream = fs.createWriteStream(path.join(root, "captured.log"), options);
          streams.push(stream); return stream;
        }
      };
      if (name === "node:child_process") return {
        spawn(executable, args, options) { spawnCalls.push({ executable, args, options }); return child; }, exec() {}
      };
      if (name === "./helpers.cjs") return { ...helpers,
        resolveDesktopDataDir: () => dataDir, resolveResourcesDir: () => resourcesDir,
        resolveBackendConfigPath: directory => path.join(directory, "application.yaml"),
        findFreePort: async () => 8999, waitBackendReady: async () => true
      };
      if (name === "./updater.cjs") return { createUpdater: () => ({}), UPDATE_STATUS_EVENT: "updater-status" };
      return require(name.startsWith(".") ? path.resolve(mainDir, name) : name);
    }
  });
  try {
    // Expose only the launched promise so the test awaits real startup completion.
    const source = fs.readFileSync(path.join(mainDir, "index.cjs"), "utf8")
      .replace("void bootstrap();", "globalThis.startupPromise = bootstrap();");
    vm.runInContext(source, context, { filename: "index.cjs" });
    await Promise.resolve(); await context.startupPromise;
    assert.equal(spawnCalls.length, 1);
    const launch = spawnCalls[0];
    assert.equal(launch.options.cwd, dataDir);
    assert.equal(launch.options.env.TRON_BACKEND_APPLICATION_CONFIG_PATH, path.join(dataDir, "application.yaml"));
    assert.equal(logCalls[0], path.join(dataDir, "logs/tron-backend.log"));
    assert.equal(launch.options.env.TRON_BACKEND_DATABASE_URL, path.join(dataDir, "tron.sqlite3"));
    assert.equal(windows[0].urls.at(-1), "http://127.0.0.1:8999");
    assert.equal(windows[0].shown, true);
    assert.equal(windows[0].focused, true);
  } finally {
    handlers.get("before-quit")?.();
    await Promise.all(streams.map(stream => finished(stream)));
    fs.rmSync(root, { recursive: true, force: true });
  }
});
