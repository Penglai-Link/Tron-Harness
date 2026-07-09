const fs = require("node:fs");
const path = require("node:path");
const { spawn } = require("node:child_process");
const { app, BrowserWindow, WebContentsView, ipcMain, shell } = require("electron");

const {
  HOST,
  buildGitHubOAuthRedirectUri,
  checkTargetPageEmbeddingStatus,
  findFreePort,
  isHttpUrl,
  loadPort,
  resolveDesktopDataDir,
  resolveFrontendDistDir,
  resolveResourcesDir,
  resolveSidecarPath,
  resolveTronDatabasePath,
  sanitizeUrl,
  waitBackendReady
} = require("./helpers.cjs");

const MAIN_WINDOW_LABEL = "tron-main";
const ALLOWED_INJECTION_EVENTS = new Set([
  "tron:frame-ready",
  "tron:dom-event",
  "tron:node-inspected",
  "tron:inspection-exited",
  "tron:navigation-requested"
]);

const injectionTemplate = fs.readFileSync(
  path.resolve(__dirname, "..", "injection", "tron-desktop-injection.js"),
  "utf8"
);

const state = {
  backendPort: 0,
  backendUrl: "",
  backendProcess: null,
  backendReady: false,
  databasePath: "",
  frontendDistDir: "",
  graphPath: "",
  injectionEnabled: false,
  mainWindow: null,
  targetViews: new Map()
};

function buildInjectionScript(pageId) {
  const injectionFlag = state.injectionEnabled ? "true" : "false";
  return injectionTemplate
    .replace("__TRON_APP_ORIGIN__", JSON.stringify(state.backendUrl))
    .replace("__TRON_PAGE_ID__", JSON.stringify(pageId))
    .replace("__TRON_INJECTION_ENABLED__", injectionFlag);
}

function createMainWindow() {
  const window = new BrowserWindow({
    height: 920,
    show: true,
    title: "Tron",
    width: 1440,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      preload: path.resolve(__dirname, "..", "preload", "app-preload.cjs"),
      sandbox: false
    }
  });

  state.mainWindow = window;
  window.on("closed", () => {
    if (state.mainWindow === window) {
      state.mainWindow = null;
    }
  });

  return window;
}

function showStartupMessage(message) {
  if (!state.mainWindow) {
    return;
  }
  const html = `<pre style="padding:24px;font-family:monospace;white-space:pre-wrap;">${escapeHtml(message)}</pre>`;
  void state.mainWindow.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(html)}`);
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function startBackend(resourcesDir) {
  const sidecarPath = resolveSidecarPath(resourcesDir);
  const child = spawn(sidecarPath, [], {
    env: {
      ...process.env,
      TRON_BACKEND_DATABASE_URL: state.databasePath,
      TRON_BACKEND_FRONTEND_DIST_DIR: state.frontendDistDir,
      TRON_BACKEND_GRAPH_STORAGE_PATH: state.graphPath,
      TRON_BACKEND_PORT: String(state.backendPort),
      TRON_AUTH_GITHUB_REDIRECT_URI: buildGitHubOAuthRedirectUri(state.backendUrl)
    },
    stdio: ["ignore", "ignore", "inherit"]
  });

  state.backendProcess = child;
  child.once("exit", () => {
    if (state.backendProcess === child) {
      state.backendProcess = null;
    }
  });
}

function stopBackend() {
  if (!state.backendProcess) {
    return;
  }
  const child = state.backendProcess;
  state.backendProcess = null;
  child.kill();
}

function isDevMode() {
  return !app.isPackaged || process.env.TRON_ELECTRON_DEVTOOLS === "1";
}

async function bootstrap() {
  const resourcesDir = resolveResourcesDir(app);
  const appDataDir = resolveDesktopDataDir();
  fs.mkdirSync(appDataDir, { recursive: true });

  state.backendPort = await findFreePort(loadPort());
  state.backendUrl = `http://${HOST}:${state.backendPort}`;
  state.databasePath = resolveTronDatabasePath(appDataDir);
  state.graphPath = path.join(appDataDir, "tron.graph.grafeo");
  state.frontendDistDir = resolveFrontendDistDir(resourcesDir);

  const mainWindow = createMainWindow();
  showStartupMessage("Backend starting...");

  try {
    startBackend(resourcesDir);
  } catch (error) {
    showStartupMessage(`Backend failed to start. ${describeError(error)}`);
    return;
  }

  state.backendReady = await waitBackendReady(state.backendUrl);
  if (!state.backendReady) {
    stopBackend();
    showStartupMessage("Backend failed to start in time. Your data directory is ~/.tron. Check logs and retry.");
    return;
  }

  await mainWindow.loadURL(state.backendUrl);
  if (isDevMode()) {
    mainWindow.webContents.openDevTools({ mode: "detach" });
  }
  mainWindow.focus();
}

function describeError(error) {
  if (error instanceof Error && error.message) {
    return error.message;
  }
  return String(error || "unknown error");
}

function normalizeRequest(payload) {
  if (payload && typeof payload === "object" && payload.request && typeof payload.request === "object") {
    return payload.request;
  }
  return payload && typeof payload === "object" ? payload : {};
}

function nextPageId() {
  return `target-${Date.now()}`;
}

function sanitizeBounds(rawBounds) {
  if (!rawBounds || typeof rawBounds !== "object") {
    return { x: 0, y: 0, width: 0, height: 0 };
  }

  return {
    height: Math.max(0, Math.round(Number(rawBounds.height) || 0)),
    width: Math.max(0, Math.round(Number(rawBounds.width) || 0)),
    x: Math.round(Number(rawBounds.x) || 0),
    y: Math.round(Number(rawBounds.y) || 0)
  };
}

function createTargetView(pageId, url, bounds) {
  if (!state.mainWindow || state.mainWindow.isDestroyed()) {
    throw new Error("main window is not available");
  }

  const view = new WebContentsView({
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      preload: path.resolve(__dirname, "..", "preload", "target-preload.cjs"),
      sandbox: false,
      webSecurity: true
    }
  });

  const target = {
    bounds: sanitizeBounds(bounds),
    pageId,
    view
  };
  state.targetViews.set(pageId, target);
  state.mainWindow.contentView.addChildView(view);
  view.setBounds(target.bounds);

  view.webContents.setWindowOpenHandler(({ url: childUrl }) => {
    const targetUrl = sanitizeUrl(childUrl);
    if (isHttpUrl(targetUrl)) {
      void view.webContents.loadURL(targetUrl);
    }
    return { action: "deny" };
  });

  view.webContents.on("did-finish-load", () => {
    void injectTargetView(target);
  });
  view.webContents.on("destroyed", () => {
    state.targetViews.delete(pageId);
  });

  void view.webContents.loadURL(url);
  if (process.env.TRON_ELECTRON_TARGET_DEVTOOLS === "1") {
    view.webContents.openDevTools({ mode: "detach" });
  }
  return target;
}

async function injectTargetView(target) {
  if (!target || target.view.webContents.isDestroyed()) {
    return;
  }
  const script = buildInjectionScript(target.pageId);
  try {
    await target.view.webContents.executeJavaScript(script, true);
  } catch (error) {
    console.error("target injection failed", error);
  }
}

function resizeTargetView(pageId, bounds) {
  const target = state.targetViews.get(pageId);
  if (!target || target.view.webContents.isDestroyed()) {
    return false;
  }

  target.bounds = sanitizeBounds(bounds);
  target.view.setBounds(target.bounds);
  return true;
}

function closeTargetView(pageId) {
  const target = state.targetViews.get(pageId);
  if (!target) {
    return false;
  }

  if (state.mainWindow && !state.mainWindow.isDestroyed()) {
    state.mainWindow.contentView.removeChildView(target.view);
  }
  if (!target.view.webContents.isDestroyed()) {
    target.view.webContents.close();
  }
  state.targetViews.delete(pageId);
  return true;
}

function sanitizeInjectionEvent(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("invalid event payload format");
  }
  const eventType = typeof raw.eventType === "string" ? raw.eventType : "";
  if (!ALLOWED_INJECTION_EVENTS.has(eventType)) {
    throw new Error("event type is not allowed");
  }
  if (typeof raw.pageId !== "string" || !raw.pageId) {
    throw new Error("event payload missing pageId");
  }
  if (typeof raw.frameId !== "string" || !raw.frameId) {
    throw new Error("event payload missing frameId");
  }
  return raw;
}

function readAnnotationRect(payload) {
  const rect = payload.rect && typeof payload.rect === "object" ? payload.rect : null;
  if (!rect) {
    return null;
  }

  return {
    height: Math.max(1, Math.round(Number(rect.height) || 1)),
    width: Math.max(1, Math.round(Number(rect.width) || 1)),
    x: Math.max(0, Math.round(Number(rect.x) || 0)),
    y: Math.max(0, Math.round(Number(rect.y) || 0))
  };
}

function buildAnnotatedScreenshotDataUrl(screenshot, payload) {
  if (!screenshot || typeof screenshot.getSize !== "function" || typeof screenshot.toDataURL !== "function") {
    return null;
  }

  const sourceSize = screenshot.getSize();
  const maxPreviewEdge = 1800;
  const sourceWidth = Math.max(1, sourceSize.width);
  const sourceHeight = Math.max(1, sourceSize.height);
  const scale = Math.min(1, maxPreviewEdge / Math.max(sourceWidth, sourceHeight));
  const image = scale < 1
    ? screenshot.resize({
      height: Math.max(1, Math.round(sourceHeight * scale)),
      quality: "best",
      width: Math.max(1, Math.round(sourceWidth * scale))
    })
    : screenshot;
  const size = image.getSize();
  const width = Math.max(1, size.width);
  const height = Math.max(1, size.height);
  const markerNumber = Number(payload.markerNumber || 1);

  return {
    captureSource: "webcontents-view",
    dataUrl: image.toDataURL(),
    height,
    markerNumber,
    mimeType: "image/png",
    sourceHeight,
    sourceWidth,
    width
  };
}

function buildInstallAnnotationOverlayScript(payload) {
  const rect = readAnnotationRect(payload);
  if (!rect) {
    return null;
  }

  const options = {
    markerNumber: Number(payload.markerNumber || 1),
    rect
  };

  return `
(() => {
  const options = ${JSON.stringify(options)};
  const overlayId = "tron-native-annotation-overlay";
  document.getElementById(overlayId)?.remove();
  const root = document.createElement("div");
  root.id = overlayId;
  root.setAttribute("data-tron-inspector-ui", "true");
  Object.assign(root.style, {
    position: "fixed",
    inset: "0",
    pointerEvents: "none",
    zIndex: "2147483647",
    contain: "layout style paint"
  });

  const rect = options.rect;
  const box = document.createElement("div");
  Object.assign(box.style, {
    position: "fixed",
    left: Math.round(rect.x) + "px",
    top: Math.round(rect.y) + "px",
    width: Math.max(1, Math.round(rect.width)) + "px",
    height: Math.max(1, Math.round(rect.height)) + "px",
    background: "rgba(22, 119, 255, 0.12)",
    border: "3px solid #1677ff",
    boxSizing: "border-box"
  });

  const markerRadius = 18;
  const markerX = Math.max(markerRadius, Math.min(window.innerWidth - markerRadius, rect.x + markerRadius + 4));
  const markerY = Math.max(markerRadius, Math.min(window.innerHeight - markerRadius, rect.y - markerRadius - 8));
  const marker = document.createElement("div");
  marker.textContent = String(options.markerNumber || 1);
  Object.assign(marker.style, {
    position: "fixed",
    left: Math.round(markerX - markerRadius) + "px",
    top: Math.round(markerY - markerRadius) + "px",
    width: markerRadius * 2 + "px",
    height: markerRadius * 2 + "px",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    borderRadius: "999px",
    background: "#1677ff",
    color: "#ffffff",
    font: "700 20px system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif",
    lineHeight: markerRadius * 2 + "px",
    boxSizing: "border-box",
    boxShadow: "0 8px 18px rgba(22, 119, 255, 0.32)"
  });

  root.append(box, marker);
  document.documentElement.appendChild(root);
  return true;
})()
`;
}

function removeAnnotationOverlayScript() {
  return `document.getElementById("tron-native-annotation-overlay")?.remove(); true;`;
}

async function waitForCaptureReadiness(sender) {
  await sender.executeJavaScript(`
new Promise((resolve) => {
  const fontsReady = document.fonts?.ready?.catch?.(() => undefined) || Promise.resolve();
  fontsReady.then(() => requestAnimationFrame(() => requestAnimationFrame(resolve)));
})
`, true);
}

async function withNativeAnnotationImage(event, sender) {
  if (event.eventType !== "tron:node-inspected") {
    return event;
  }
  const payload = event.payload && typeof event.payload === "object" ? { ...event.payload } : {};

  if (!sender || sender.isDestroyed()) {
    return { ...event, payload };
  }

  const installScript = buildInstallAnnotationOverlayScript(payload);
  if (installScript) {
    try {
      await sender.executeJavaScript(installScript, true);
      await waitForCaptureReadiness(sender);
    } catch (error) {
      console.error("native target annotation overlay failed", error);
    }
  }

  try {
    const image = await sender.capturePage();
    if (!image || typeof image.isEmpty !== "function" || image.isEmpty()) {
      return { ...event, payload };
    }
    const annotationImage = buildAnnotatedScreenshotDataUrl(image, payload);
    if (annotationImage) {
      payload.annotationImage = annotationImage;
    }
  } catch (error) {
    console.error("native target capture failed", error);
  } finally {
    try {
      await sender.executeJavaScript(removeAnnotationOverlayScript(), true);
    } catch (_error) {}
  }

  return { ...event, payload };
}

function forwardTargetEvent(event) {
  if (!state.mainWindow || state.mainWindow.isDestroyed()) {
    return;
  }
  state.mainWindow.webContents.send("desktop-target-event", event);
}

function registerIpcHandlers() {
  ipcMain.handle("tron-desktop:invoke", async (_event, command, payload = {}) => {
    const request = normalizeRequest(payload);

    if (command === "desktopBackendStatus") {
      return {
        backendUrl: state.backendUrl,
        backendPort: state.backendPort,
        ready: state.backendReady,
        injectionEnabled: state.injectionEnabled,
        databasePath: state.databasePath,
        graphPath: state.graphPath
      };
    }

    if (command === "checkTargetPageEmbedding") {
      return checkTargetPageEmbeddingStatus(request.url);
    }

    if (command === "openTargetPage") {
      const pageId = String(request.pageId || nextPageId());
      const mode = String(request.mode || "iframe");
      const url = sanitizeUrl(request.url);
      if (!isHttpUrl(url)) {
        throw new Error("Only http/https URLs are allowed");
      }

      if (mode === "browser") {
        await shell.openExternal(url);
        return { pageId, opened: true, mode: "browser", backendUrl: state.backendUrl };
      }

      if (mode !== "view" && mode !== "window") {
        return {
          pageId,
          opened: false,
          mode,
          reason: "requested-iframe-mode",
          backendUrl: state.backendUrl
        };
      }

      const existing = state.targetViews.get(pageId);
      if (existing && !existing.view.webContents.isDestroyed()) {
        resizeTargetView(pageId, request.bounds);
        if (existing.view.webContents.getURL() !== url) {
          void existing.view.webContents.loadURL(url);
        }
      } else {
        createTargetView(pageId, url, request.bounds);
      }
      return { pageId, opened: true, mode: "view", backendUrl: state.backendUrl };
    }

    if (command === "closeTargetPage") {
      const pageId = String(request.pageId || "");
      return { closed: closeTargetView(pageId) };
    }

    if (command === "resizeTargetPageView") {
      const pageId = String(request.pageId || "");
      return { resized: resizeTargetView(pageId, request.bounds) };
    }

    if (command === "setInjectionEnabled") {
      state.injectionEnabled = Boolean(request.enabled);
      for (const [pageId, target] of state.targetViews) {
        if (!target.view.webContents.isDestroyed()) {
          void injectTargetView(target);
          target.view.webContents.send("tron-desktop:toggle-injection", {
            enabled: state.injectionEnabled,
            pageId
          });
        }
      }
      return state.injectionEnabled;
    }

    if (command === "desktopInjectionEvent") {
      sanitizeInjectionEvent(request.event);
      return undefined;
    }

    throw new Error(`unknown desktop command: ${command}`);
  });

  ipcMain.handle("tron-desktop:target-event", async (ipcEvent, rawEvent) => {
    const sanitized = sanitizeInjectionEvent(rawEvent);
    const withImage = await withNativeAnnotationImage(sanitized, ipcEvent.sender);
    forwardTargetEvent(withImage);
    return true;
  });
}

registerIpcHandlers();

app.whenReady().then(() => {
  void bootstrap();
});

app.on("window-all-closed", () => {
  app.quit();
});

app.on("before-quit", () => {
  stopBackend();
});
