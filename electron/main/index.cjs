const fs = require("node:fs");
const path = require("node:path");
const readline = require("node:readline");
const { spawn, exec } = require("node:child_process");
const { app, BrowserWindow, clipboard, ipcMain, Menu, shell } = require("electron");

const {
  HOST,
  buildGitHubOAuthRedirectUri,
  checkTargetPageEmbeddingStatus,
  findFreePort,
  isHttpUrl,
  loadPort,
  resolveBackendConfigPath,
  resolveBackendLogPath,
  resolveDesktopDataDir,
  resolveFrontendDistDir,
  resolveResourcesDir,
  resolveSidecarPath,
  resolveTronDatabasePath,
  sanitizeUrl,
  waitBackendReady
} = require("./helpers.cjs");
const {
  createWebviewCommentPayload,
  dispatchWebviewComment,
  buildWebviewContextMenuTemplate
} = require("./webview-context-menu.cjs");
const { handleFsCommand } = require("./fs-gateway.cjs");

const MAIN_WINDOW_LABEL = "tron-main";
const MIN_STARTUP_SCREEN_MS = 1200;
const ALLOWED_INJECTION_EVENTS = new Set([
  "tron:frame-ready",
  "tron:dom-event",
  "tron:inspection-started",
  "tron:node-inspected",
  "tron:inspection-exited",
  "tron:navigation-requested",
  "tron:navigation-changed"
]);
const commentCapablePageIdsByWebContentsId = new Map();
let isQuitting = false;

function setCommentAvailability(webContentsId, pageId, available) {
  if (available) {
    commentCapablePageIdsByWebContentsId.set(webContentsId, pageId);
  } else {
    commentCapablePageIdsByWebContentsId.delete(webContentsId);
  }
}

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
  backendLogPath: "",
  frontendDistDir: "",
  graphPath: "",
  injectionEnabled: false,
  mainWindow: null
};

function buildWebviewInjectionScript(pageId, enabled) {
  return injectionTemplate
    .replace("__TRON_APP_ORIGIN__", JSON.stringify(state.backendUrl))
    .replace("__TRON_PAGE_ID__", JSON.stringify(pageId))
    .replace("__TRON_INJECTION_ENABLED__", enabled ? "true" : "false");
}

function createMainWindow() {
  const targetPreloadPath = path.resolve(__dirname, "..", "preload", "target-preload.cjs");
  const window = new BrowserWindow({
    height: 920,
    show: false,
    title: "Tron",
    width: 1440,
    webPreferences: {
      webviewTag: true,
      contextIsolation: true,
      nodeIntegration: false,
      preload: path.resolve(__dirname, "..", "preload", "app-preload.cjs"),
      sandbox: false
    }
  });

  state.mainWindow = window;

  window.on("close", (event) => {
    if (process.platform !== "darwin" || isQuitting) {
      return;
    }

    event.preventDefault();
    window.hide();
  });

  window.webContents.on("will-attach-webview", (_event, webPreferences) => {
    webPreferences.contextIsolation = true;
    webPreferences.nodeIntegration = false;
    webPreferences.preload = targetPreloadPath;
    webPreferences.sandbox = false;
    webPreferences.webSecurity = true;
  });

  window.webContents.on("did-attach-webview", (_event, guestContents) => {
    guestContents.setWindowOpenHandler(({ url }) => {
      const targetUrl = sanitizeUrl(url);
      if (isHttpUrl(targetUrl)) {
        void guestContents.loadURL(targetUrl);
      }
      return { action: "deny" };
    });

    guestContents.on("context-menu", (event, params) => {
      event.preventDefault();
      if (guestContents.isDestroyed()) {
        return;
      }

      const template = buildWebviewContextMenuTemplate(guestContents, params, {
        addComment: (x, y) => {
          const pageId = commentCapablePageIdsByWebContentsId.get(guestContents.id);
          if (!pageId) {
            return;
          }

          void dispatchWebviewComment(
            guestContents.mainFrame,
            createWebviewCommentPayload(pageId, x, y)
          ).then((dispatched) => {
            if (!dispatched) {
              console.error("failed to dispatch webview comment command");
            }
          });
        },
        allowInspect: isDevMode(),
        canComment: commentCapablePageIdsByWebContentsId.has(guestContents.id),
        copyText: (value) => clipboard.writeText(value),
        isHttpUrl,
        locale: app.getLocale(),
        openExternal: openUrlInSystemBrowser
      });
      Menu.buildFromTemplate(template).popup({
        frame: params.frame || undefined,
        sourceType: params.menuSourceType,
        window
      });
    });

    guestContents.once("destroyed", () => {
      commentCapablePageIdsByWebContentsId.delete(guestContents.id);
    });
  });

  window.webContents.setWindowOpenHandler(({ url }) => {
    const targetUrl = sanitizeUrl(url);
    if (!isHttpUrl(targetUrl)) {
      return { action: "deny" };
    }
    openUrlInSystemBrowser(targetUrl);
    return { action: "deny" };
  });

  window.on("closed", () => {
    if (state.mainWindow === window) {
      state.mainWindow = null;
    }
  });

  return window;
}

async function showStartupMessage(message) {
  if (!state.mainWindow) {
    return;
  }
  const html = startupPageHtml(message);
  await state.mainWindow.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(html)}`);
}

async function keepStartupScreenVisible(startedAt) {
  const remainingMs = MIN_STARTUP_SCREEN_MS - (Date.now() - startedAt);
  if (remainingMs > 0) {
    await new Promise((resolve) => setTimeout(resolve, remainingMs));
  }
}

function startupPageHtml(message) {
  const safeMessage = escapeHtml(message).replace(/\r?\n/g, "<br />");
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Tron</title>
    <style>
      :root {
        color-scheme: light;
        font-family: Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, "SF Pro Display", sans-serif;
        background: #f3f5f8;
        color: #101827;
      }

      * { box-sizing: border-box; }

      html, body { height: 100%; margin: 0; }

      body {
        display: grid;
        min-height: 100%;
        place-items: center;
        padding: 32px;
        overflow: hidden;
        background:
          radial-gradient(circle at center, rgba(255, 255, 255, 0.1) 0%, rgba(247, 250, 252, 0.48) 52%, rgba(238, 243, 248, 0.82) 100%),
          #eef3f8;
      }

      body::before {
        position: fixed;
        inset: 0;
        background-image: radial-gradient(rgba(142, 165, 214, 0.12) 1px, transparent 1px);
        background-size: 8px 8px;
        content: "";
        mask-image: radial-gradient(circle at center, black, transparent 76%);
        pointer-events: none;
      }

      .status {
        position: relative;
        width: min(100%, 420px);
        padding: 32px;
        border: 1px solid rgba(16, 24, 39, 0.08);
        border-radius: 28px;
        background: rgba(255, 255, 255, 0.72);
        box-shadow: 0 18px 48px rgba(46, 54, 79, 0.08);
        animation: appear 420ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
      }

      .status-kicker {
        color: #8992a2;
        font-size: 10px;
        font-weight: 750;
        letter-spacing: 0.18em;
      }

      .status-row {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-top: 20px;
        color: #101827;
        font-size: 14px;
      }

      .status-dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        background: #6f60e8;
        box-shadow: 0 0 0 5px rgba(111, 96, 232, 0.12);
        animation: pulse 1.8s ease-in-out infinite;
      }

      .status-message {
        max-height: 86px;
        margin-top: 14px;
        overflow: auto;
        color: #687386;
        font-family: "SF Mono", "Roboto Mono", monospace;
        font-size: 11px;
        line-height: 1.65;
        white-space: normal;
      }

      .progress {
        position: relative;
        height: 4px;
        margin-top: 24px;
        overflow: hidden;
        border-radius: 10px;
        background: rgba(16, 24, 39, 0.09);
      }

      .progress::after {
        position: absolute;
        inset: 0;
        width: 42%;
        border-radius: inherit;
        background: linear-gradient(90deg, transparent, #6f60e8, #9ff1ff, transparent);
        content: "";
        animation: sweep 2.1s ease-in-out infinite;
      }

      .meta {
        display: grid;
        gap: 8px;
        margin-top: 26px;
        color: #8992a2;
        font-size: 11px;
      }

      .meta-row {
        display: flex;
        justify-content: space-between;
        gap: 20px;
      }

      .meta code {
        color: #687386;
        font-family: "SF Mono", "Roboto Mono", monospace;
        font-size: 10px;
      }

      @keyframes appear { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
      @keyframes pulse { 0%, 100% { opacity: 0.45; transform: scale(0.82); } 50% { opacity: 1; transform: scale(1); } }
      @keyframes sweep { 0% { transform: translateX(-130%); } 60%, 100% { transform: translateX(310%); } }

      @media (max-width: 760px) {
        body { padding: 24px; }
        .status { padding: 24px; }
      }

      @media (prefers-reduced-motion: reduce) {
        *, *::before, *::after { animation-duration: 0.01ms !important; animation-iteration-count: 1 !important; }
      }
    </style>
  </head>
  <body>
    <section class="status" aria-live="polite">
      <div class="status-row"><span class="status-dot"></span><strong>Connecting to Tron backend</strong></div>
      <div class="status-message">${safeMessage}</div>
      <div class="progress" aria-hidden="true"></div>
      <div class="meta">
        <div class="meta-row"><span>Data directory</span><code>~/.tron</code></div>
        <div class="meta-row"><span>Backend logs</span><code>~/.tron/logs/</code></div>
      </div>
    </section>
  </body>
</html>`;
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function startBackend(resourcesDir) {
  const sidecarPath = resolveSidecarPath(resourcesDir);
  const appDataDir = path.dirname(state.databasePath);
  const configPath = resolveBackendConfigPath(appDataDir, resourcesDir);
  const logPath = resolveBackendLogPath(path.dirname(state.databasePath));
  fs.mkdirSync(path.dirname(logPath), { recursive: true });
  const logStream = fs.createWriteStream(logPath, { flags: "a" });
  logStream.on("error", (error) => {
    console.error(`tron-backend log stream failed: ${describeError(error)}`);
  });
  const writeLog = (source, message) => {
    const line = `[${new Date().toISOString()}] [${source}] ${String(message)}`;
    if (!logStream.destroyed && !logStream.writableEnded) {
      logStream.write(`${line}\n`);
    }
  };

  writeLog("lifecycle", `using application config: ${configPath}`);
  writeLog("lifecycle", `starting sidecar: ${sidecarPath}`);
  const child = spawn(sidecarPath, [], {
    env: {
      ...process.env,
      TRON_BACKEND_DATABASE_URL: state.databasePath,
      TRON_BACKEND_APPLICATION_CONFIG_PATH: configPath,
      TRON_BACKEND_FRONTEND_DIST_DIR: state.frontendDistDir,
      TRON_BACKEND_GRAPH_STORAGE_PATH: state.graphPath,
      TRON_BACKEND_PORT: String(state.backendPort),
      TRON_AUTH_GITHUB_REDIRECT_URI: buildGitHubOAuthRedirectUri(state.backendUrl)
    },
    cwd: appDataDir,
    stdio: ["ignore", "pipe", "pipe"]
  });

  const stdout = readline.createInterface({ input: child.stdout });
  const stderr = readline.createInterface({ input: child.stderr });
  stdout.on("line", (line) => writeLog("stdout", line));
  stderr.on("line", (line) => writeLog("stderr", line));
  child.once("error", (error) => writeLog("lifecycle", `process error: ${describeError(error)}`));
  child.once("close", (code, signal) => {
    writeLog("lifecycle", `exited with code=${code ?? "null"} signal=${signal ?? "null"}`);
    stdout.close();
    stderr.close();
    logStream.end();
  });

  state.backendProcess = child;
  state.backendLogPath = logPath;
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

function shouldOpenMainDevTools() {
  return process.env.TRON_ELECTRON_DEVTOOLS === "1";
}

async function bootstrap() {
  const resourcesDir = resolveResourcesDir(app);
  const appDataDir = resolveDesktopDataDir();
  fs.mkdirSync(appDataDir, { recursive: true });

  state.backendPort = await findFreePort(loadPort());
  state.backendUrl = `http://${HOST}:${state.backendPort}`;
  state.databasePath = resolveTronDatabasePath(appDataDir);
  state.backendLogPath = resolveBackendLogPath(appDataDir);
  state.graphPath = path.join(appDataDir, "tron.graph.grafeo");
  state.frontendDistDir = resolveFrontendDistDir(resourcesDir);

  const mainWindow = createMainWindow();
  await showStartupMessage("Backend starting...");
  mainWindow.show();
  const startupScreenStartedAt = Date.now();

  try {
    startBackend(resourcesDir);
  } catch (error) {
    await showStartupMessage(`Backend failed to start. ${describeError(error)}\nLogs: ${state.backendLogPath}`);
    return;
  }

  state.backendReady = await waitBackendReady(state.backendUrl);
  if (!state.backendReady) {
    stopBackend();
    await showStartupMessage(`Backend failed to start in time. Check logs at ${state.backendLogPath} and retry.`);
    return;
  }

  await keepStartupScreenVisible(startupScreenStartedAt);
  await mainWindow.loadURL(state.backendUrl);
  if (shouldOpenMainDevTools()) {
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

function isWSL() {
  try {
    return /microsoft/i.test(fs.readFileSync("/proc/version", "utf8"));
  } catch {
    return false;
  }
}

// ShellExecuteEx 的 P/Invoke 声明，供 WSL 场景经 PowerShell 调用
const TRON_SHELL_OPEN_CSHARP = `using System;
using System.Runtime.InteropServices;
public static class TronShellOpen
{
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  private struct SHELLEXECUTEINFO
  {
    public int cbSize;
    public uint fMask;
    public IntPtr hwnd;
    [MarshalAs(UnmanagedType.LPWStr)] public string lpVerb;
    [MarshalAs(UnmanagedType.LPWStr)] public string lpFile;
    [MarshalAs(UnmanagedType.LPWStr)] public string lpParameters;
    [MarshalAs(UnmanagedType.LPWStr)] public string lpDirectory;
    public int nShow;
    public IntPtr hInstApp;
    public IntPtr lpIDList;
    [MarshalAs(UnmanagedType.LPWStr)] public string lpClass;
    public IntPtr hkeyClass;
    public uint dwHotKey;
    public IntPtr hIcon;
    public IntPtr hProcess;
  }

  [DllImport("shell32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  private static extern bool ShellExecuteEx(ref SHELLEXECUTEINFO info);

  public static void Open(string target)
  {
    SHELLEXECUTEINFO info = new SHELLEXECUTEINFO();
    info.cbSize = Marshal.SizeOf(typeof(SHELLEXECUTEINFO));
    info.fMask = 0x04000000u; // SEE_MASK_FLAG_LOG_USAGE
    info.lpFile = target;
    info.nShow = 1; // SW_SHOWNORMAL
    if (!ShellExecuteEx(ref info))
    {
      throw new Exception("ShellExecuteEx failed: " + Marshal.GetLastWin32Error());
    }
  }
}`;

function buildWslPowerShellOpenCommand(url) {
  const safeUrl = url.replace(/'/g, "''");
  const script = `$csharp = @'
${TRON_SHELL_OPEN_CSHARP}
'@
Add-Type -TypeDefinition $csharp
[TronShellOpen]::Open('${safeUrl}')`;
  const encoded = Buffer.from(script, "utf16le").toString("base64");
  return `powershell.exe -NoProfile -NonInteractive -EncodedCommand ${encoded}`;
}

function releaseMainWindowFocus() {
  const window = state.mainWindow;
  if (!window || window.isDestroyed() || !window.isFocused()) {
    return;
  }
  window.blur();
}

function openUrlInSystemBrowser(url) {
  const escapedUrl = url.replace(/"/g, "");
  if (isWSL()) {
    releaseMainWindowFocus();
    exec(buildWslPowerShellOpenCommand(url), { windowsHide: true }, (error) => {
      if (!error) {
        return;
      }
      exec(`cmd.exe /c start "" "${escapedUrl}"`, { windowsHide: true }, (cmdError) => {
        if (cmdError) {
          exec(`wslview "${escapedUrl}"`, { windowsHide: true }, (wslviewError) => {
            if (wslviewError) {
              shell.openExternal(url).catch(() => {});
            }
          });
        }
      });
    });
    return;
  }

  if (process.platform === "win32") {
    releaseMainWindowFocus();
    shell.openExternal(url, { logUsage: true }).catch(() => {
      exec(`start "" "${escapedUrl}"`, { windowsHide: true }, () => {});
    });
    return;
  }

  // macOS / 原生 Linux
  shell.openExternal(url).catch(() => {});
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

    if (typeof command === "string" && command.startsWith("fs:")) {
      const browserWindow = BrowserWindow.fromWebContents(_event.sender) || undefined;
      return handleFsCommand(command, request, browserWindow);
    }

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
        // fire-and-forget：立即返回，不等浏览器打开
        // 避免渲染进程后续处理抢回焦点导致浏览器窗口层级变低
        openUrlInSystemBrowser(url);
        return { pageId, opened: true, mode: "browser", backendUrl: state.backendUrl };
      }

      return {
        pageId,
        opened: false,
        mode,
        reason: "target-pages-use-webview",
        backendUrl: state.backendUrl
      };
    }

    if (command === "desktopInjectionEvent") {
      sanitizeInjectionEvent(request.event);
      return undefined;
    }

    throw new Error(`unknown desktop command: ${command}`);
  });

  ipcMain.handle("tron-desktop:target-event", async (ipcEvent, rawEvent) => {
    const sanitized = sanitizeInjectionEvent(rawEvent);
    if (process.env.TRON_ELECTRON_TARGET_DEBUG === "1" && sanitized.eventType !== "tron:dom-event") {
      console.log(`[target-debug] page event eventType=${sanitized.eventType} pageId=${sanitized.pageId}`);
    }
    const withImage = await withNativeAnnotationImage(sanitized, ipcEvent.sender);
    forwardTargetEvent(withImage);
    return true;
  });

  ipcMain.handle("tron-desktop:target-injection-script", (ipcEvent, payload = {}) => {
    if (!state.mainWindow || state.mainWindow.isDestroyed()) {
      return "";
    }

    const host = ipcEvent.sender.hostWebContents;
    if (!host || host.id !== state.mainWindow.webContents.id) {
      return "";
    }

    const pageId = String(payload.pageId || "").trim().slice(0, 160);
    if (!pageId) {
      return "";
    }

    setCommentAvailability(ipcEvent.sender.id, pageId, payload.commentAvailable);

    return buildWebviewInjectionScript(pageId, Boolean(payload.enabled));
  });

  ipcMain.handle("tron-desktop:update-target-comment-availability", (ipcEvent, payload = {}) => {
    if (!state.mainWindow || state.mainWindow.isDestroyed()) {
      return false;
    }

    const host = ipcEvent.sender.hostWebContents;
    if (!host || host.id !== state.mainWindow.webContents.id) {
      return false;
    }

    const pageId = String(payload.pageId || "").trim().slice(0, 160);
    if (!pageId) {
      return false;
    }

    setCommentAvailability(ipcEvent.sender.id, pageId, payload.commentAvailable);

    return true;
  });
}

registerIpcHandlers();

app.whenReady().then(() => {
  void bootstrap();
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", () => {
  if (!state.mainWindow || state.mainWindow.isDestroyed()) {
    void bootstrap();
    return;
  }

  state.mainWindow.show();
  state.mainWindow.focus();
});

app.on("before-quit", () => {
  isQuitting = true;
  stopBackend();
});
