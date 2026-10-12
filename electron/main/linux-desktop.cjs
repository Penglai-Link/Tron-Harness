const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const { ICON_PATH } = require("./desktop-constants.cjs");
const { createLinuxAutostart } = require("./linux-autostart.cjs");

function hasStatusNotifierHost(run = execFileSync) {
  try {
    const result = run("gdbus", [
      "call", "--session", "--dest", "org.kde.StatusNotifierWatcher",
      "--object-path", "/StatusNotifierWatcher",
      "--method", "org.freedesktop.DBus.Properties.Get",
      "org.kde.StatusNotifierWatcher", "IsStatusNotifierHostRegistered"
    ], { encoding: "utf8", timeout: 2000, stdio: ["ignore", "pipe", "ignore"] });
    return /\btrue\b/.test(result);
  } catch { return false; }
}

function createLinuxDesktop({ app, Tray, Menu, showWindow, quit, dataDir,
  startup,
  hasTrayHost = hasStatusNotifierHost }) {
  let tray = null, watcher = null;
  let startupAvailable = false;

  function refreshMenu() {
    if (!tray) return;
    let enabled = false;
    try { enabled = startupAvailable && startup.isEnabled(); }
    catch (error) { console.error("failed to read Linux startup setting", error); }
    tray.setContextMenu(Menu.buildFromTemplate([
      { label: "显示窗口", click: showWindow },
      {
        label: "开机启动", type: "checkbox", checked: Boolean(enabled), enabled: startupAvailable,
        click: item => {
          try { startup.setEnabled(item.checked); }
          catch (error) { console.error("failed to change Linux startup setting", error); }
          // Linux requires replacing the context menu after a checkbox changes.
          refreshMenu();
        }
      },
      { type: "separator" },
      { label: "退出", click: quit }
    ]));
  }

  return {
    initialize() {
      try {
        if (startup === undefined && app.isPackaged) startup = createLinuxAutostart({ dataDir });
        startup?.initialize();
        startupAvailable = Boolean(startup);
      }
      catch (error) {
        startupAvailable = false;
        console.error("failed to initialize Linux startup setting", error);
      }
      // A Tray object can exist on GNOME even when no panel displays its icon.
      // Without an indicator host, keep normal window/quit behavior available.
      if (!hasTrayHost()) return;
      try {
        tray = new Tray(ICON_PATH);
        tray.setToolTip("Tron");
        tray.on("click", showWindow);
        refreshMenu();
      } catch (error) {
        console.error("failed to initialize Linux tray", error);
        tray?.destroy();
        tray = null;
        return;
      }
      if (startupAvailable) {
        try {
          watcher = fs.watch(path.dirname(startup.filename), { persistent: false }, (_event, filename) => {
            if (!filename || String(filename) === path.basename(startup.filename)) refreshMenu();
          });
          watcher.on("error", error => console.error("Linux startup settings watcher failed", error));
        } catch (error) { console.error("failed to watch Linux startup setting", error); }
      }
    },
    isResident: () => Boolean(tray),
    dispose() {
      watcher?.close();
      watcher = null;
      tray?.destroy();
      tray = null;
    }
  };
}

module.exports = { createLinuxDesktop, hasStatusNotifierHost };
