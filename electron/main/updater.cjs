const fs = require("node:fs");
const fsp = require("node:fs/promises");
const os = require("node:os");
const path = require("node:path");
const { spawn: defaultSpawn } = require("node:child_process");
const { DEFAULT_PERSON_AI_SERVER_BASE_URL } = require("./helpers.cjs");

const PRODUCT_CODE = "tron";
const UPDATE_STATUS_EVENT = "update:status-changed";

const OS_TYPES = Object.freeze({
  WINDOWS_X86: 110,
  WINDOWS_X64: 120,
  WINDOWS_ARM64: 130,
  MACOS_X64: 220,
  MACOS_ARM64: 230,
  LINUX_X86: 310,
  LINUX_X64: 320,
  LINUX_ARM64: 330,
  GENERIC: 0
});

const UPDATE_TYPES = new Set([
  "idle",
  "checking",
  "upToDate",
  "updateAvailable",
  "downloading",
  "downloaded",
  "installing",
  "error"
]);

function currentOsType(platformName = process.platform, archName = process.arch) {
  if (platformName === "win32") {
    if (archName === "x64") return OS_TYPES.WINDOWS_X64;
    if (archName === "arm64") return OS_TYPES.WINDOWS_ARM64;
    if (archName === "ia32") return OS_TYPES.WINDOWS_X86;
  }

  if (platformName === "darwin") {
    if (archName === "x64") return OS_TYPES.MACOS_X64;
    if (archName === "arm64") return OS_TYPES.MACOS_ARM64;
  }

  if (platformName === "linux") {
    if (archName === "x64") return OS_TYPES.LINUX_X64;
    if (archName === "arm64") return OS_TYPES.LINUX_ARM64;
    if (archName === "ia32") return OS_TYPES.LINUX_X86;
  }

  return OS_TYPES.GENERIC;
}

function versionSegments(value) {
  return String(value || "")
    .trim()
    .replace(/^[vV]/, "")
    .split(".")
    .map((segment) => {
      const digits = segment.match(/^\d+/)?.[0];
      return digits ? Number.parseInt(digits, 10) : 0;
    });
}

function compareVersions(left, right) {
  const leftSegments = versionSegments(left);
  const rightSegments = versionSegments(right);
  const length = Math.max(leftSegments.length, rightSegments.length);

  for (let index = 0; index < length; index += 1) {
    const leftValue = leftSegments[index] ?? 0;
    const rightValue = rightSegments[index] ?? 0;
    if (leftValue !== rightValue) {
      return leftValue > rightValue ? 1 : -1;
    }
  }

  return 0;
}

function normalizeVersion(value) {
  const version = String(value || "").trim().replace(/^[vV]/, "");
  if (!version || !/^\d+(?:\.\d+)*(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
    return "";
  }
  return version;
}

function normalizeBaseUrl(value) {
  const raw = String(value || "").trim();
  if (!raw) {
    throw new Error("TRON_VERSION_SERVER_URL is not configured");
  }

  const url = new URL(raw);
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error("TRON_VERSION_SERVER_URL must use http or https");
  }

  url.pathname = url.pathname.replace(/\/+$/, "");
  if (url.pathname.endsWith("/api/v1")) {
    url.pathname = url.pathname.slice(0, -"/api/v1".length);
  }
  return url;
}

function buildApiUrl(baseUrl, suffix) {
  const url = normalizeBaseUrl(baseUrl);
  const basePath = url.pathname.replace(/\/+$/, "");
  const [pathname, query] = String(suffix || "").split("?", 2);
  url.pathname = `${basePath}/api/v1${pathname}`;
  url.search = query ? `?${query}` : "";
  return url;
}

function readResponseError(response, body, operation) {
  const message = body && typeof body.message === "string" ? body.message : `${operation} failed with HTTP ${response.status}`;
  const error = new Error(message);
  error.status = response.status;
  error.code = body && (typeof body.code === "string" || typeof body.code === "number") ? String(body.code) : undefined;
  return error;
}

function unwrapApiData(body, response, operation) {
  if (!body || typeof body !== "object" || Array.isArray(body)) {
    return body;
  }

  if (!("code" in body) || !("data" in body)) {
    return body;
  }

  const code = body.code;
  const success = code === 0 || code === 200 || code === "0" || code === "200" || code === "SUCCESS";
  if (!success) {
    throw readResponseError(response, body, operation);
  }
  return body.data;
}

async function readJsonResponse(response, operation) {
  const body = await response.json().catch(() => null);
  if (!response.ok) {
    throw readResponseError(response, body, operation);
  }
  return unwrapApiData(body, response, operation);
}

function sanitizeFileName(value) {
  const fileName = path.basename(String(value || "")).replace(/[\u0000-\u001f]/g, "").trim();
  return fileName && fileName !== "." && fileName !== ".." ? fileName : "";
}

function installerExtension(platformName) {
  if (platformName === "win32") return ".exe";
  if (platformName === "darwin") return ".dmg";
  return ".AppImage";
}

function resolveInstallerFileName(packageInfo, version, platformName) {
  const expectedExtension = installerExtension(platformName);
  const configuredName = sanitizeFileName(packageInfo?.file_name ?? packageInfo?.fileName);
  if (configuredName && configuredName.toLowerCase().endsWith(expectedExtension.toLowerCase())) {
    return configuredName;
  }
  return `Tron_${version}${expectedExtension}`;
}

function describeError(error) {
  if (error instanceof Error && error.message) return error.message;
  return String(error || "unknown error");
}

function cloneStatus(status) {
  return { ...status };
}

function defaultDataDir() {
  return path.join(os.homedir(), ".tron");
}

function defaultUpdaterLogPath(dataDir) {
  return path.join(dataDir, "logs", "tron-updater.log");
}

async function writeLog(logPath, message) {
  try {
    await fsp.mkdir(path.dirname(logPath), { recursive: true });
    await fsp.appendFile(logPath, `${new Date().toISOString()} ${message}\n`, "utf8");
  } catch (_error) {
    // Logging must never prevent the update helper from completing.
  }
}

async function waitForLauncherExit(input = process.stdin) {
  if (!input || typeof input.once !== "function") return;
  await new Promise((resolve) => {
    input.once("end", resolve);
    input.once("close", resolve);
    if (typeof input.resume === "function") input.resume();
  });
}

function waitForWriterDrain(writer) {
  return new Promise((resolve, reject) => {
    const cleanup = () => {
      writer.removeListener("drain", onDrain);
      writer.removeListener("error", onError);
    };
    const onDrain = () => {
      cleanup();
      resolve();
    };
    const onError = (error) => {
      cleanup();
      reject(error);
    };

    writer.once("drain", onDrain);
    writer.once("error", onError);
  });
}

async function runApplyUpdate(installerPath, options = {}) {
  const platformName = options.platformName ?? process.platform;
  const spawnImpl = options.spawnImpl ?? defaultSpawn;
  const input = options.input ?? process.stdin;
  const logPath = options.logPath ?? defaultUpdaterLogPath(defaultDataDir());
  const filePath = String(installerPath || "");

  if (!filePath) {
    await writeLog(logPath, "update helper: installer path is missing");
    return 1;
  }

  try {
    await writeLog(logPath, `update helper started: ${filePath}`);
    await waitForLauncherExit(input);

    if (platformName !== "win32") {
      await writeLog(logPath, `update helper does not support platform ${platformName}`);
      return 1;
    }

    const child = spawnImpl(filePath, ["/S", "/R"], {
      detached: true,
      stdio: "ignore",
      windowsHide: true
    });
    child.unref?.();
    await writeLog(logPath, "update helper spawned NSIS installer");
    return 0;
  } catch (error) {
    await writeLog(logPath, `update helper failed: ${describeError(error)}`);
    return 1;
  }
}

class Updater {
  constructor(options = {}) {
    this.dataDir = options.dataDir ?? defaultDataDir();
    this.getVersion = options.getVersion ?? (() => "0.0.0");
    this.getVersionServerUrl = options.getVersionServerUrl ?? (() => process.env.TRON_VERSION_SERVER_URL || DEFAULT_PERSON_AI_SERVER_BASE_URL);
    this.platformName = options.platformName ?? process.platform;
    this.archName = options.archName ?? process.arch;
    this.fetchImpl = options.fetchImpl ?? globalThis.fetch?.bind(globalThis);
    this.spawnImpl = options.spawnImpl ?? defaultSpawn;
    this.openInstaller = options.openInstaller ?? (async () => "desktop-open-not-configured");
    this.exit = options.exit ?? (() => {});
    this.onStatus = options.onStatus ?? (() => {});
    this.status = { type: "idle" };
    this.update = null;
    this.downloadPromise = null;
    this.logPath = options.logPath ?? defaultUpdaterLogPath(this.dataDir);
  }

  getStatus() {
    return cloneStatus(this.status);
  }

  setStatus(status) {
    if (!UPDATE_TYPES.has(status.type)) {
      throw new Error(`unknown update status: ${status.type}`);
    }
    this.status = cloneStatus(status);
    this.onStatus(this.getStatus());
    return this.getStatus();
  }

  requireFetch() {
    if (typeof this.fetchImpl !== "function") {
      throw new Error("fetch is not available in the Electron runtime");
    }
    return this.fetchImpl;
  }

  requireToken(accessToken) {
    const token = String(accessToken || "").trim();
    if (!token) throw new Error("access token is required for update checks");
    return token;
  }

  async requestJson(suffix, token, operation) {
    const fetchImpl = this.requireFetch();
    const url = buildApiUrl(this.getVersionServerUrl(), suffix);
    const response = await fetchImpl(url, {
      headers: {
        Authorization: `Bearer ${token}`,
        "X-Product-Code": PRODUCT_CODE,
        Accept: "application/json"
      }
    });
    return readJsonResponse(response, operation);
  }

  async check(accessToken) {
    if (this.status.type === "checking") return this.getStatus();
    const token = this.requireToken(accessToken);
    this.setStatus({ type: "checking" });

    try {
      const fetchImpl = this.requireFetch();
      const url = buildApiUrl(this.getVersionServerUrl(), `/versions/latest?product_code=${encodeURIComponent(PRODUCT_CODE)}`);
      const response = await fetchImpl(url, {
        headers: {
          Authorization: `Bearer ${token}`,
          "X-Product-Code": PRODUCT_CODE,
          Accept: "application/json"
        }
      });

      if (response.status === 404) {
        this.update = null;
        return this.setStatus({ type: "upToDate" });
      }

      const latest = await readJsonResponse(response, "checking for updates");
      const version = normalizeVersion(latest?.version_number ?? latest?.versionNumber);
      if (!version) throw new Error("version service returned no valid version number");

      if (compareVersions(version, this.getVersion()) <= 0) {
        this.update = null;
        return this.setStatus({ type: "upToDate" });
      }

      const osType = currentOsType(this.platformName, this.archName);
      const packages = Array.isArray(latest?.packages) ? latest.packages : [];
      const packageInfo = packages.find((item) => Number(item?.os_type ?? item?.osType) === osType);
      if (!packageInfo) throw new Error("no update package is available for the current platform");

      const versionId = String(latest?.id || "").trim();
      if (!versionId) throw new Error("version service returned no version id");

      this.update = {
        version,
        versionId,
        packageInfo,
        fileName: resolveInstallerFileName(packageInfo, version, this.platformName)
      };
      return this.setStatus({
        type: "updateAvailable",
        version,
        versionId,
        releaseDate: String(latest?.release_date ?? latest?.releaseDate ?? ""),
        fileName: this.update.fileName
      });
    } catch (error) {
      this.update = null;
      this.setStatus({ type: "error", message: describeError(error) });
      throw error;
    }
  }

  async startDownload(accessToken) {
    if (this.downloadPromise) return true;
    if (this.status.type !== "updateAvailable" || !this.update) {
      throw new Error("no downloadable update is available");
    }

    const token = this.requireToken(accessToken);
    this.downloadPromise = this.download(token).finally(() => {
      this.downloadPromise = null;
    });
    await this.downloadPromise;
    return true;
  }

  async download(token) {
    const update = this.update;
    this.setStatus({ type: "downloading", version: update.version, progress: 0 });

    const updatesDir = path.join(this.dataDir, "updates");
    const finalPath = path.join(updatesDir, update.fileName);
    const partialPath = `${finalPath}.part`;

    try {
      const downloadPayload = await this.requestJson(
        `/versions/${encodeURIComponent(update.versionId)}/download?os_type=${currentOsType(this.platformName, this.archName)}`,
        token,
        "requesting update download"
      );
      const downloadUrl = String(downloadPayload?.download_url ?? downloadPayload?.downloadUrl ?? "").trim();
      if (!downloadUrl) throw new Error("version service returned no download URL");

      const downloadResponse = await this.requireFetch()(downloadUrl, { headers: { Accept: "application/octet-stream" } });
      if (!downloadResponse.ok) {
        throw readResponseError(downloadResponse, null, "downloading update");
      }
      if (!downloadResponse.body || typeof downloadResponse.body.getReader !== "function") {
        throw new Error("update download response has no readable body");
      }

      await fsp.mkdir(updatesDir, { recursive: true });
      await fsp.rm(partialPath, { force: true });
      const writer = fs.createWriteStream(partialPath, { flags: "wx" });
      let writerError = null;
      writer.on("error", (error) => {
        writerError = error;
      });
      const reader = downloadResponse.body.getReader();
      const total = Number(downloadResponse.headers?.get?.("content-length") || 0);
      let downloaded = 0;

      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          if (!value || value.length === 0) continue;
          downloaded += value.length;
          if (!writer.write(Buffer.from(value))) await waitForWriterDrain(writer);
          if (total > 0) {
            this.setStatus({
              type: "downloading",
              version: update.version,
              progress: Math.min(100, (downloaded / total) * 100)
            });
          }
        }
        if (writerError) throw writerError;
        await new Promise((resolve, reject) => {
          writer.once("error", reject);
          writer.end(resolve);
        });
      } catch (error) {
        await new Promise((resolve) => {
          if (writer.closed) {
            resolve();
            return;
          }
          writer.once("close", resolve);
          writer.once("error", resolve);
          writer.destroy();
        });
        throw error;
      }

      await fsp.rm(finalPath, { force: true });
      await fsp.rename(partialPath, finalPath);
      this.setStatus({
        type: "downloaded",
        version: update.version,
        filePath: finalPath,
        fileName: update.fileName
      });
      return this.getStatus();
    } catch (error) {
      await fsp.rm(partialPath, { force: true }).catch(() => {});
      this.setStatus({ type: "error", message: describeError(error) });
      throw error;
    }
  }

  async apply() {
    if (this.status.type !== "downloaded") throw new Error("no downloaded update is available");
    const version = this.status.version;
    try {
      const filePath = String(this.status.filePath || "");
      if (!filePath || !fs.existsSync(filePath)) throw new Error("downloaded update file is missing");

      if (this.platformName === "darwin") {
        const openError = await this.openInstaller(filePath);
        if (openError) throw new Error(String(openError));
        this.setStatus({ type: "installing", version });
        setTimeout(() => this.exit(0), 600).unref?.();
        return true;
      }

      if (this.platformName !== "win32") throw new Error(`automatic installation is not supported on ${this.platformName}`);

      const child = this.spawnImpl(process.execPath, ["--_tron-apply-update", filePath, this.logPath], {
        detached: true,
        stdio: ["pipe", "ignore", "ignore"]
      });
      child.unref?.();
      child.stdin?.unref?.();
      this.setStatus({ type: "installing", version });
      setTimeout(() => this.exit(0), 600).unref?.();
      return true;
    } catch (error) {
      this.setStatus({ type: "error", version, message: describeError(error) });
      throw error;
    }
  }
}

function createUpdater(options) {
  return new Updater(options);
}

module.exports = {
  PRODUCT_CODE,
  UPDATE_STATUS_EVENT,
  OS_TYPES,
  Updater,
  buildApiUrl,
  compareVersions,
  createUpdater,
  currentOsType,
  normalizeVersion,
  resolveInstallerFileName,
  runApplyUpdate,
  waitForLauncherExit
};
