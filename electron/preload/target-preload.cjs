const { contextBridge, ipcRenderer, webFrame } = require("electron");

let configuredPageId = "";
let configureGeneration = 0;
let disposed = false;

async function configureTarget(payload) {
  const generation = ++configureGeneration;
  const pageId = typeof payload?.pageId === "string" ? payload.pageId.trim().slice(0, 160) : "";
  if (!pageId || disposed) {
    return;
  }

  configuredPageId = pageId;
  const enabled = Boolean(payload?.enabled);
  const script = await ipcRenderer.invoke("tron-desktop:target-injection-script", {
    enabled,
    pageId
  });
  if (
    disposed
    || generation !== configureGeneration
    || typeof script !== "string"
    || configuredPageId !== pageId
  ) {
    return;
  }

  await webFrame.executeJavaScript(script, true);
  if (disposed || generation !== configureGeneration) {
    return;
  }
  window.postMessage(
    {
      channel: "tron:frame-event",
      type: "tron:toggle-injection",
      pageId,
      enabled
    },
    "*"
  );
}

window.addEventListener("message", (event) => {
  if (disposed) {
    return;
  }
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
  if (disposed || (configuredPageId && payload?.pageId !== configuredPageId)) {
    return;
  }

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

ipcRenderer.on("tron-desktop:configure-target", (_event, payload) => {
  void configureTarget(payload).catch(() => undefined);
});

window.addEventListener("unload", () => {
  disposed = true;
  configureGeneration += 1;
  ipcRenderer.removeAllListeners("tron-desktop:configure-target");
  ipcRenderer.removeAllListeners("tron-desktop:toggle-injection");
}, { once: true });

contextBridge.exposeInMainWorld("tronDesktopTarget", {
  ready: true
});
