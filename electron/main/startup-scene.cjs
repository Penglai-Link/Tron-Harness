const fs = require("node:fs");
const path = require("node:path");

// Presentation only: retain the live startup scene while the main document loads.
function createStartupScene({ window, WebContentsView, ipcMain, dataDir, onBeforeInput }) {
  const themePath = path.join(dataDir, "startup-theme.json");
  let theme = "matrix-dark";
  let firstStartup = true;
  let pendingTheme = null;
  try {
    const stored = JSON.parse(fs.readFileSync(themePath, "utf8"));
    if (["light", "matrix-dark"].includes(stored)) {
      theme = stored;
      firstStartup = false;
    }
  } catch {}
  const syncWindowTheme = () => {
    window.setBackgroundColor(theme === "light" ? "#ffffff" : "#020305");
    if (process.platform !== "darwin") {
      window.setTitleBarOverlay({ color: "#00000000", symbolColor: theme === "light" ? "#202124" : "#f3f1eb" });
    }
  };
  syncWindowTheme();
  let view = null;
  let fallbackTimer = null;
  const resize = () => {
    const [width, height] = window.getContentSize();
    view?.setBounds({ x: 0, y: 0, width, height });
  };
  const dismiss = (initialized = false) => {
    if (fallbackTimer) clearTimeout(fallbackTimer);
    fallbackTimer = null;
    if (!view) return;
    if (!window.isDestroyed()) window.contentView.removeChildView(view);
    if (!view.webContents.isDestroyed()) view.webContents.close();
    view = null;
    if (initialized && firstStartup) {
      firstStartup = false;
      if (pendingTheme) applyTheme(pendingTheme);
      pendingTheme = null;
    }
  };
  const ready = (event) => { if (event.sender === window.webContents) dismiss(true); };
  const applyTheme = (value) => {
    const changed = theme !== value;
    theme = value;
    syncWindowTheme();
    if (changed && view) {
      view.setBackgroundColor(theme === "light" ? "#ffffff" : "#020305");
      void view.webContents.executeJavaScript(`document.documentElement.dataset.theme = ${JSON.stringify(theme)}`).catch(() => {});
    }
    try {
      fs.mkdirSync(dataDir, { recursive: true });
      fs.writeFileSync(themePath, JSON.stringify(theme));
    } catch (error) { console.error("Could not save startup animation theme", error); }
  };
  const saveTheme = (event, value) => {
    if (event.sender !== window.webContents || !["light", "matrix-dark"].includes(value)) return;
    if (firstStartup) {
      pendingTheme = value;
      return;
    }
    applyTheme(value);
  };
  ipcMain.on("tron-animation:ready", ready);
  ipcMain.on("tron-animation:theme", saveTheme);
  window.on("resize", resize);
  window.once("closed", () => {
    dismiss();
    ipcMain.removeListener("tron-animation:ready", ready);
    ipcMain.removeListener("tron-animation:theme", saveTheme);
  });
  return {
    get theme() { return theme; },
    async show(html) {
      if (!view) {
        view = new WebContentsView({ webPreferences: { sandbox: true, contextIsolation: true, nodeIntegration: false } });
        if (onBeforeInput) view.webContents.on("before-input-event", onBeforeInput);
        view.setBackgroundColor(theme === "light" ? "#ffffff" : "#020305");
        window.contentView.addChildView(view);
        resize();
      }
      await view.webContents.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(html)}`);
    },
    dismissIfNotReadyAfter(delayMs = 5000) {
      if (!view || fallbackTimer) return;
      fallbackTimer = setTimeout(() => dismiss(true), delayMs);
    },
  };
}

module.exports = { createStartupScene };
