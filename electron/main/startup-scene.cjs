const fs = require("node:fs");
const path = require("node:path");

// Presentation only: retain the live startup scene while the main document loads.
function createStartupScene({ window, WebContentsView, ipcMain, dataDir }) {
  const themePath = path.join(dataDir, "startup-theme.json");
  let theme = "matrix-dark";
  try { if (JSON.parse(fs.readFileSync(themePath, "utf8")) === "light") theme = "light"; } catch {}
  window.setBackgroundColor(theme === "light" ? "#ffffff" : "#020305");
  let view = null;
  let fallbackTimer = null;
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
  };
  const ready = (event) => { if (event.sender === window.webContents) dismiss(); };
  const saveTheme = (event, value) => {
    if (event.sender !== window.webContents || !["light", "matrix-dark"].includes(value)) return;
    const changed = theme !== value;
    theme = value;
    window.setBackgroundColor(theme === "light" ? "#ffffff" : "#020305");
    if (changed && view) {
      view.setBackgroundColor(theme === "light" ? "#ffffff" : "#020305");
      void view.webContents.executeJavaScript(`document.documentElement.dataset.theme = ${JSON.stringify(theme)}`).catch(() => {});
    }
    try {
      fs.mkdirSync(dataDir, { recursive: true });
      fs.writeFileSync(themePath, JSON.stringify(theme));
    } catch (error) { console.error("Could not save startup animation theme", error); }
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
        view.setBackgroundColor(theme === "light" ? "#ffffff" : "#020305");
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
