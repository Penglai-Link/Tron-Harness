const { contextBridge, ipcRenderer } = require("electron");
const path = require("node:path");
const { pathToFileURL } = require("node:url");

contextBridge.exposeInMainWorld("tronDesktop", {
  targetPreloadUrl: pathToFileURL(path.resolve(__dirname, "target-preload.cjs")).href,
  invoke(command, payload) {
    return ipcRenderer.invoke("tron-desktop:invoke", command, payload || {});
  }
});

ipcRenderer.on("desktop-target-event", (_event, payload) => {
  window.postMessage(payload, window.location.origin);
});
