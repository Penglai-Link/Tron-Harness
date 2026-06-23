const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("tronDesktop", {
  invoke(command, payload) {
    return ipcRenderer.invoke("tron-desktop:invoke", command, payload || {});
  }
});

ipcRenderer.on("desktop-target-event", (_event, payload) => {
  window.postMessage(payload, window.location.origin);
});
