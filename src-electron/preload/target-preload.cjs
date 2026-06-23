const { contextBridge, ipcRenderer } = require("electron");

window.addEventListener("message", (event) => {
  const payload = event.data;
  if (!payload || typeof payload !== "object") {
    return;
  }

  if (payload.channel !== "tron:frame-event") {
    return;
  }

  if (payload.type === "tron:toggle-injection" || typeof payload.eventType !== "string") {
    return;
  }

  void ipcRenderer.invoke("tron-desktop:target-event", payload);
});

ipcRenderer.on("tron-desktop:toggle-injection", (_event, payload) => {
  window.postMessage(
    {
      channel: "tron:frame-event",
      type: "tron:toggle-injection",
      pageId: payload?.pageId,
      enabled: Boolean(payload?.enabled)
    },
    "*"
  );
});

contextBridge.exposeInMainWorld("tronDesktopTarget", {
  ready: true
});
