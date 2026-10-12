const path = require("node:path");

const APP_ID = "com.tron.desktop";
const STARTUP_ARG = "--tron-startup";
const ICON_PATH = path.resolve(__dirname, "..", "assets", "tron.png");

function createWindowsDesktop({ app, Tray, Menu, showWindow, quit, execPath = process.execPath }) {
  let tray = null;
  const loginOptions = { path: execPath, args: [STARTUP_ARG] };

  function refreshMenu() {
    const settings = app.isPackaged ? app.getLoginItemSettings(loginOptions) : null;
    const enabled = Boolean(settings?.openAtLogin && settings?.executableWillLaunchAtLogin);
    tray.setContextMenu(Menu.buildFromTemplate([
      { label: "显示窗口", click: showWindow },
      {
        label: "开机启动", type: "checkbox", checked: enabled, enabled: app.isPackaged,
        click: (item) => {
          try {
            app.setLoginItemSettings({
              ...loginOptions, name: APP_ID, openAtLogin: item.checked, enabled: item.checked
            });
          } catch (error) {
            console.error("failed to change Windows startup setting", error);
          }
          refreshMenu();
        }
      },
      { type: "separator" },
      { label: "退出", click: quit }
    ]));
  }

  return {
    initialize() {
      app.setAppUserModelId(APP_ID);
      try {
        tray = new Tray(ICON_PATH);
        tray.setToolTip("Tron");
        tray.on("click", showWindow);
        tray.on("double-click", showWindow);
        // Re-read Windows settings before opening the menu, including changes
        // made in Task Manager while Tron remains in the background.
        tray.on("right-click", () => {
          refreshMenu();
        });
        refreshMenu();
      } catch (error) {
        console.error("failed to initialize Windows tray", error);
        tray?.destroy();
        tray = null;
      }
    },
    isResident: () => Boolean(tray),
    dispose() {
      tray?.destroy();
      tray = null;
    }
  };
}

module.exports = { APP_ID, STARTUP_ARG, ICON_PATH, createWindowsDesktop };
