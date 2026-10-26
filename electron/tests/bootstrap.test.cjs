const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const vm = require("node:vm");
const { EventEmitter, once } = require("node:events");
const { PassThrough } = require("node:stream");
const { finished } = require("node:stream/promises");

// Run the actual bootstrap with desktop/sidecar boundaries replaced. No GUI,
// persistent product files, network request or real subprocess is created.
for (const scenario of [
  { name: "host development", platform: process.platform, packaged: false },
  { name: "Windows manual launch", platform: "win32", packaged: true },
  { name: "Windows startup launch", platform: "win32", packaged: true, startup: true },
  { name: "manual launch during Windows startup", platform: "win32", packaged: true, startup: true, earlyRestore: true },
  { name: "Windows session end", platform: "win32", packaged: true, sessionEnd: true },
  { name: "Windows tray failure", platform: "win32", packaged: true, startup: true, trayFailure: true },
  { name: "Linux manual launch", platform: "linux", packaged: true },
  { name: "Linux startup launch", platform: "linux", packaged: true, startup: true },
  { name: "manual launch during Linux startup", platform: "linux", packaged: true, startup: true, earlyRestore: true },
  { name: "Linux without an indicator host", platform: "linux", packaged: true, startup: true, noTrayHost: true },
  { name: "Linux tray failure", platform: "linux", packaged: true, startup: true, trayFailure: true }
]) {
test(`bootstrap and lifecycle: ${scenario.name}`, async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tron-bootstrap-"));
  const dataDir = path.join(root, "data");
  const resourcesDir = path.join(root, "resources");
  fs.mkdirSync(path.join(resourcesDir, "binaries"), { recursive: true });
  fs.writeFileSync(path.join(resourcesDir, "binaries", process.platform === "win32" ? "tron-backend.exe" : "tron-backend"), "fixture only");
  const spawnCalls = [], logCalls = [], streams = [], windows = [], trays = [], loginCalls = [];
  const electronPaths = new Map(), startupEvents = [], configOptions = [];
  let quitCalls = 0, killCalls = 0;
  let loginEnabled = true;
  let notifyStartupChange = () => {};
  const handlers = new Map();
  const mainDir = path.resolve(__dirname, "../main");
  const helpers = require("../main/helpers.cjs");
  const linuxStartup = require("../main/linux-autostart.cjs").createLinuxAutostart({
    dataDir, env: { XDG_CONFIG_HOME: path.join(root, "config") }, execPath: "/opt/Tron/tron"
  });
  let clock = 0;
  class FastClock extends Date { static now() { return clock += 2000; } }
  class FakeWindow extends EventEmitter {
    constructor() {
      super(); windows.push(this); this.urls = [];
      this.webContents = new EventEmitter();
      this.webContents.setWindowOpenHandler = () => {};
      this.webContents.openDevTools = () => {};
      this.contentView = { addChildView() {}, removeChildView() {} };
    }
    async loadURL(url) { this.urls.push(url); }
    show() { this.shown = true; }
    hide() { this.shown = false; }
    focus() { this.focused = true; }
    isDestroyed() { return false; }
    isVisible() { return Boolean(this.shown); }
    isMinimized() { return Boolean(this.minimized); }
    restore() { this.minimized = false; }
    setBackgroundColor() {}
    getContentSize() { return [1440, 920]; }
  }
  class FakeView {
    constructor() { this.webContents = { loadURL: async () => {}, isDestroyed: () => false, close() {} }; }
    setBounds() {}
    setBackgroundColor() {}
  }
  class FakeTray extends EventEmitter {
    constructor() {
      super();
      if (scenario.trayFailure) throw new Error("tray unavailable");
      trays.push(this);
    }
    setToolTip() {}
    setContextMenu(menu) { this.menu = menu; this.emit("menu-updated"); }
    destroy() { this.destroyed = true; }
  }
  const child = new EventEmitter();
  child.stdout = new PassThrough(); child.stderr = new PassThrough();
  child.kill = () => { killCalls++; child.emit("close", 0, null); return true; };
  const context = vm.createContext({
    console, setTimeout, clearTimeout, Date: FastClock, __dirname: mainDir,
    process: { argv: scenario.startup ? ["--tron-startup"] : [], env: {}, platform: scenario.platform },
    require(name) {
      if (name === "electron") return {
        app: {
          isPackaged: scenario.packaged,
          getLocale: () => "en-US",
          requestSingleInstanceLock: () => { startupEvents.push("lock"); return true; },
          setPath(name, directory) { electronPaths.set(name, directory); startupEvents.push(name); },
          setAppUserModelId() {},
          setName() {},
          getLoginItemSettings: () => ({ openAtLogin: loginEnabled, executableWillLaunchAtLogin: loginEnabled }),
          setLoginItemSettings(settings) { loginCalls.push(settings); loginEnabled = settings.openAtLogin; },
          quit() { quitCalls++; handlers.get("before-quit")?.(); },
          whenReady: () => Promise.resolve(), on: (name, cb) => handlers.set(name, cb)
        },
        BrowserWindow: FakeWindow, WebContentsView: FakeView, Tray: FakeTray, ipcMain: { handle() {}, on() {}, removeListener() {} }, clipboard: {},
        Menu: { buildFromTemplate: template => template }, shell: {}
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
        resolveBackendConfigPath: (directory, _resources, options) => { configOptions.push(options); return path.join(directory, "application.yaml"); },
        findFreePort: async () => 8999, waitBackendReady: async () => true
      };
      if (name === "./updater.cjs") return { createUpdater: () => ({}), UPDATE_STATUS_EVENT: "updater-status" };
      if (name === "./linux-desktop.cjs") return {
        createLinuxDesktop: options => {
          const desktop = require("../main/linux-desktop.cjs").createLinuxDesktop({
            ...options, startup: scenario.packaged ? linuxStartup : null,
            hasTrayHost: () => !scenario.noTrayHost
          });
          const initialize = desktop.initialize;
          desktop.initialize = () => {
            const watch = fs.watch;
            // Deliver the filesystem boundary explicitly; macOS can coalesce
            // rapid writes to the simulated Linux startup file.
            fs.watch = (_directory, _options, listener) => {
              notifyStartupChange = () => listener("change", path.basename(linuxStartup.filename));
              const watcher = new EventEmitter();
              watcher.close = () => {};
              return watcher;
            };
            try { initialize(); } finally { fs.watch = watch; }
          };
          return desktop;
        }
      };
      return require(name.startsWith(".") ? path.resolve(mainDir, name) : name);
    }
  });
  try {
    // Expose only the launched promise so the test awaits real startup completion.
    const source = fs.readFileSync(path.join(mainDir, "index.cjs"), "utf8")
      .replace("void bootstrap();", "globalThis.startupPromise = bootstrap();");
    vm.runInContext(source, context, { filename: "index.cjs" });
    if (scenario.earlyRestore) handlers.get("second-instance")({}, ["Tron.exe"]);
    await Promise.resolve(); await context.startupPromise;
    assert.equal(windows.length, 1);
    assert.equal(spawnCalls.length, 1);
    const launch = spawnCalls[0];
    assert.equal(launch.options.cwd, dataDir);
    assert.equal(launch.options.env.TRON_BACKEND_APPLICATION_CONFIG_PATH, path.join(dataDir, "application.yaml"));
    assert.equal(logCalls[0], path.join(dataDir, "logs/tron-backend.log"));
    assert.equal(launch.options.env.TRON_BACKEND_DATABASE_URL, path.join(dataDir, "tron.sqlite3"));
    assert.equal(launch.options.env.TRON_BACKEND_CONTROL_DATABASE_URL, path.join(dataDir, "control/profiles.sqlite"));
    assert.equal(launch.options.env.TRON_BACKEND_GRAPH_STORAGE_PATH, path.join(dataDir, "tron.graph.grafeo"));
    const installLocal = scenario.platform === "win32" && scenario.packaged;
    assert.equal(configOptions[0].relocateStorage, installLocal);
    if (installLocal) {
      assert.equal(electronPaths.get("userData"), path.join(dataDir, "electron"));
      assert.equal(electronPaths.get("sessionData"), path.join(dataDir, "electron"));
      assert.equal(electronPaths.get("logs"), path.join(dataDir, "logs"));
      assert.equal(electronPaths.get("crashDumps"), path.join(dataDir, "crash-dumps"));
      assert.ok(startupEvents.indexOf("sessionData") < startupEvents.indexOf("lock"));
    } else {
      assert.equal(electronPaths.size, 0);
    }
    assert.equal(windows[0].urls.at(-1), "http://127.0.0.1:8999");
    const background = scenario.startup && !scenario.trayFailure && !scenario.earlyRestore && !scenario.noTrayHost;
    assert.equal(Boolean(windows[0].shown), !background);
    assert.equal(Boolean(windows[0].focused), !background);
    assert.equal(launch.options.windowsHide, true);
    if (scenario.sessionEnd) {
      windows[0].emit("session-end");
      assert.equal(killCalls, 1);
      let prevented = false;
      windows[0].emit("close", { preventDefault() { prevented = true; } });
      assert.equal(prevented, false, "logoff must close the window");
    } else if (["win32", "linux"].includes(scenario.platform) && !scenario.trayFailure && !scenario.noTrayHost && scenario.packaged) {
      const window = windows[0], tray = trays[0];
      let prevented = false;
      window.emit("close", { preventDefault() { prevented = true; } });
      assert.equal(prevented, true);
      assert.equal(window.shown, false);
      assert.equal(killCalls, 0, "hiding must keep the sidecar running");
      window.minimized = true;
      tray.emit("click");
      assert.equal(window.shown, true);
      assert.equal(window.minimized, false);
      assert.equal(window.focused, true);
      window.hide();
      handlers.get("second-instance")({}, ["Tron.exe", "--tron-startup"]);
      assert.equal(window.shown, false, "duplicate startup should stay in background");
      handlers.get("second-instance")({}, ["Tron.exe"]);
      assert.equal(window.shown, true);
      assert.equal(spawnCalls.length, 1, "restoring must not launch another sidecar");
      assert.equal(loginCalls.length, 0, "normal startup must preserve OS startup preferences");
      tray.menu.find(item => item.type === "checkbox").click({ checked: false });
      if (scenario.platform === "win32") {
        assert.equal(loginCalls.at(-1).openAtLogin, false);
        assert.equal(loginCalls.at(-1).name, "com.tron.desktop");
        assert.deepEqual(loginCalls.at(-1).args, ["--tron-startup"]);
        // Simulate enabling startup in Windows Settings while Tron is running.
        loginEnabled = true;
        tray.emit("right-click");
      } else {
        assert.equal(linuxStartup.isEnabled(), false);
        // Desktop startup settings modify the same file; the menu follows it.
        const menuUpdated = once(tray, "menu-updated");
        linuxStartup.setEnabled(true);
        notifyStartupChange();
        await menuUpdated;
        assert.equal(loginCalls.length, 0, "Linux must not use Windows login APIs");
      }
      assert.equal(tray.menu.find(item => item.type === "checkbox").checked, true);
      tray.menu.find(item => item.type === "checkbox").click({ checked: true });
      if (scenario.platform === "win32") assert.equal(loginCalls.at(-1).enabled, true);
      else assert.equal(linuxStartup.isEnabled(), true);
      handlers.get("window-all-closed")();
      assert.equal(quitCalls, 0);
      tray.menu.find(item => item.label === "退出").click();
      assert.equal(quitCalls, 1);
      assert.equal(killCalls, 1);
      assert.equal(tray.destroyed, true);
      prevented = false;
      window.emit("close", { preventDefault() { prevented = true; } });
      assert.equal(prevented, false, "explicit quit must allow the window to close");
    } else if (scenario.trayFailure || scenario.noTrayHost) {
      let prevented = false;
      windows[0].emit("close", { preventDefault() { prevented = true; } });
      assert.equal(prevented, false, "tray failure must not leave an unreachable background app");
      handlers.get("window-all-closed")();
      assert.equal(quitCalls, 1);
    }
  } finally {
    handlers.get("before-quit")?.();
    await Promise.all(streams.map(stream => finished(stream)));
    fs.rmSync(root, { recursive: true, force: true });
  }
});
}

for (const platform of ["win32", "linux"]) {
test(`a second ${platform} process exits before registering IPC or starting a backend`, () => {
  let quitCalls = 0;
  const source = fs.readFileSync(path.resolve(__dirname, "../main/index.cjs"), "utf8");
  vm.runInNewContext(source, {
    process: { argv: [], platform },
    require(name) {
      if (name === "electron") return {
        app: { requestSingleInstanceLock: () => false, quit: () => quitCalls++ }
      };
      return require(name.startsWith(".") ? path.resolve(__dirname, "../main", name) : name);
    }
  });
  assert.equal(quitCalls, 1);
});
}
