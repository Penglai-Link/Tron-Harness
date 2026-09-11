const { contextBridge, ipcRenderer, webFrame } = require("electron");
const {
  buildPageMessageScript,
  normalizeTargetConfiguration
} = require("./target-preload-helpers.cjs");

let configuredPageId = "";
let configureGeneration = 0;
let disposed = false;

function postPageMessage(payload) {
  if (disposed) {
    return Promise.resolve();
  }

  return webFrame
    .executeJavaScript(buildPageMessageScript(payload), true)
    .catch(() => undefined);
}

async function configureTarget(payload) {
  const generation = ++configureGeneration;
  const configuration = normalizeTargetConfiguration(payload);
  if (!configuration || disposed) {
    return;
  }

  configuredPageId = configuration.pageId;
  const script = await ipcRenderer.invoke(
    "tron-desktop:target-injection-script",
    configuration
  );
  if (
    disposed
    || generation !== configureGeneration
    || typeof script !== "string"
    || configuredPageId !== configuration.pageId
  ) {
    return;
  }

  await webFrame.executeJavaScript(script, true);
  if (disposed || generation !== configureGeneration) {
    return;
  }
  await postPageMessage({
    channel: "tron:frame-event",
    type: "tron:toggle-injection",
    pageId: configuration.pageId,
    enabled: configuration.enabled
  });
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

  const configuration = normalizeTargetConfiguration(payload);
  if (configuration && configuration.pageId === configuredPageId) {
    void ipcRenderer.invoke(
      "tron-desktop:update-target-comment-availability",
      configuration
    ).catch(() => undefined);
  }

  void postPageMessage({
    channel: "tron:frame-event",
    type: "tron:toggle-injection",
    pageId: payload?.pageId,
    enabled: Boolean(payload?.enabled)
  });
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
