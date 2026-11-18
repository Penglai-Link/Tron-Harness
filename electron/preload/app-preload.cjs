const { contextBridge, ipcRenderer } = require("electron");
const path = require("node:path");
const { pathToFileURL } = require("node:url");

const windowChromePlatform = { darwin: "macos", win32: "windows", linux: "linux" }[process.platform];
if (windowChromePlatform) {
  let fullscreen = false;
  const updateWindowChrome = () => {
    if (!document.documentElement) return;
    document.documentElement.dataset.tronWindowChrome = windowChromePlatform;
    document.documentElement.dataset.tronWindowFullscreen = String(fullscreen);
  };
  ipcRenderer.on("tron-desktop:window-chrome", (_event, state) => {
    fullscreen = state.fullscreen === true;
    updateWindowChrome();
  });
  window.addEventListener("DOMContentLoaded", updateWindowChrome, { once: true });
}

contextBridge.exposeInMainWorld("tronDesktop", {
  platform: process.platform,
  targetPreloadUrl: pathToFileURL(path.resolve(__dirname, "target-preload.cjs")).href,
  invoke(command, payload) {
    return ipcRenderer.invoke("tron-desktop:invoke", command, payload || {});
  },
  onUpdateStatus(listener) {
    const handler = (_event, payload) => listener(payload);
    ipcRenderer.on("update:status-changed", handler);
    return () => ipcRenderer.removeListener("update:status-changed", handler);
  },
  onBackendEnvironmentStatus(listener) {
    const handler = (_event, payload) => listener(payload);
    ipcRenderer.on("backend-environment:status", handler);
    return () => ipcRenderer.removeListener("backend-environment:status", handler);
  },
  onNotificationActivated(listener) {
    const handler = (_event, payload) => listener(payload);
    ipcRenderer.on("tron-desktop:notification-activated", handler);
    return () => ipcRenderer.removeListener("tron-desktop:notification-activated", handler);
  }
});

ipcRenderer.on("desktop-target-event", (_event, payload) => {
  window.postMessage(payload, window.location.origin);
});

require("./startup-animation.cjs")({ ipcRenderer, window, document });
