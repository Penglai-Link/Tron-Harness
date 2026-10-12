const test = require("node:test");
const assert = require("node:assert/strict");
const { EventEmitter } = require("node:events");
const { createLinuxDesktop, hasStatusNotifierHost } = require("../main/linux-desktop.cjs");

test("Linux indicator detection requires a registered host and tolerates missing session tools", () => {
  assert.equal(hasStatusNotifierHost(() => "(<true>,)"), true);
  assert.equal(hasStatusNotifierHost(() => "(<false>,)"), false);
  assert.equal(hasStatusNotifierHost(() => { throw new Error("no session bus or gdbus"); }), false);
});

test("Linux development tray never registers a startup entry", () => {
  let tray;
  class FakeTray extends EventEmitter {
    constructor() { super(); tray = this; }
    setToolTip() {}
    setContextMenu(menu) { this.menu = menu; }
    destroy() {}
  }
  const desktop = createLinuxDesktop({
    app: { isPackaged: false }, Tray: FakeTray, Menu: { buildFromTemplate: menu => menu },
    hasTrayHost: () => true, showWindow() {}, quit() {}
  });
  desktop.initialize();
  assert.equal(desktop.isResident(), true);
  assert.equal(tray.menu.find(item => item.type === "checkbox").enabled, false);
  desktop.dispose();
  assert.equal(desktop.isResident(), false);
});

test("Linux startup initialization failure leaves the tray usable", t => {
  let tray;
  class FakeTray extends EventEmitter {
    constructor() { super(); tray = this; }
    setToolTip() {}
    setContextMenu(menu) { this.menu = menu; }
    destroy() {}
  }
  let shown = false;
  t.mock.method(console, "error", () => {});
  const desktop = createLinuxDesktop({
    app: { isPackaged: true }, Tray: FakeTray, Menu: { buildFromTemplate: menu => menu },
    hasTrayHost: () => true,
    startup: { initialize() { throw new Error("read-only config directory"); } },
    showWindow() { shown = true; }, quit() {}
  });
  desktop.initialize();
  assert.equal(desktop.isResident(), true);
  assert.equal(tray.menu.find(item => item.type === "checkbox").enabled, false);
  tray.menu.find(item => item.label === "显示窗口").click();
  assert.equal(shown, true);
  desktop.dispose();
});
