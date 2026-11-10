const { startupPalette } = require("./startup-page.cjs");

// Presentation only: retain the live startup scene while the main document loads.
function createStartupScene({ window, WebContentsView, ipcMain, nativeTheme, onBeforeInput }) {
  const systemTheme = () => nativeTheme.shouldUseDarkColors ? "matrix-dark" : "light";
  let appTheme = null;
  let view = null;
  let fallbackTimer = null;
  const syncWindowTheme = () => {
    // Startup follows the OS; the app preference takes over once it is ready.
    const palette = startupPalette(view ? systemTheme() : appTheme ?? systemTheme());
    window.setBackgroundColor(palette.background);
    view?.setBackgroundColor(palette.background);
    if (process.platform !== "darwin") {
      window.setTitleBarOverlay({ color: "#00000000", symbolColor: palette.foreground });
    }
  };
  syncWindowTheme();
  const resize = () => {
    const [width, height] = window.getContentSize();
    view?.setBounds({ x: 0, y: 0, width, height });
  };
  const dismiss = () => {
    if (fallbackTimer) clearTimeout(fallbackTimer);
    fallbackTimer = null;
    if (!view) return;
    if (!window.isDestroyed()) window.contentView.removeChildView(view);
    if (!view.webContents.isDestroyed()) view.webContents.close();
    view = null;
    if (!window.isDestroyed()) syncWindowTheme();
  };
  const ready = (event) => { if (event.sender === window.webContents) dismiss(); };
  const saveTheme = (event, value) => {
    if (event.sender !== window.webContents || !["light", "matrix-dark"].includes(value)) return;
    appTheme = value;
    syncWindowTheme();
  };
  ipcMain.on("tron-animation:ready", ready);
  ipcMain.on("tron-animation:theme", saveTheme);
  nativeTheme.on("updated", syncWindowTheme);
  window.on("resize", resize);
  window.once("closed", () => {
    dismiss();
    nativeTheme.removeListener("updated", syncWindowTheme);
    ipcMain.removeListener("tron-animation:ready", ready);
    ipcMain.removeListener("tron-animation:theme", saveTheme);
  });
  return {
    async show(html) {
      if (!view) {
        view = new WebContentsView({ webPreferences: { sandbox: true, contextIsolation: true, nodeIntegration: false } });
        if (onBeforeInput) view.webContents.on("before-input-event", onBeforeInput);
        syncWindowTheme();
        window.contentView.addChildView(view);
        resize();
      }
      await view.webContents.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(html)}`);
    },
    dismissIfNotReadyAfter(delayMs = 5000) {
      if (!view || fallbackTimer) return;
      fallbackTimer = setTimeout(dismiss, delayMs);
    },
  };
}

module.exports = { createStartupScene };
