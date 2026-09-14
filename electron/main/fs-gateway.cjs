/**
 * fs-gateway.cjs
 *
 * 仅供桌面端使用的文件系统网关，服务于右侧「文件」面板。
 *
 * 渲染进程以 `nodeIntegration: false` 运行，无法直接访问本地文件系统。本模块
 * 是唯一的特权入口，负责：
 *   - 弹出系统「打开文件夹/文件」对话框（用户授权一个根目录）；
 *   - 将授权根登记到内存注册表；
 *   - 在授权根范围内执行 list / read / write / create / rename / delete。
 *
 * 每个路径操作都会基于授权根重新解析并做 containment（含 realpath）校验，
 * 从而防止被攻破的渲染进程逃逸到用户显式打开的目录之外。
 *
 * 本模块刻意只在 pick 处理器内 `require("electron")`，使纯 fs/path 工具函数
 * 可在 `node --test` 下直接进行单元测试。
 */

const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

/** 单次请求可读取文本文件的最大字节数。 */
const MAX_READ_BYTES = 2 * 1024 * 1024;
/** 单次写入请求可接受的最大字节数。 */
const MAX_WRITE_BYTES = 2 * 1024 * 1024;
/** 单个路径段允许的最大长度。 */
const MAX_PATH_LENGTH = 4096;
/** 用于二进制探测的前导字节数。 */
const BINARY_SNIFF_BYTES = 8192;

/**
 * @typedef {{ realPath: string, name: string }} FsRoot
 */

/** @type {Map<string, FsRoot>} */
const roots = new Map();

function assertString(value, field) {
  if (typeof value !== "string" || value.length === 0 || value.length > MAX_PATH_LENGTH) {
    throw new Error(`invalid-${field}`);
  }
  return value;
}

function assertRelativePath(value) {
  if (typeof value !== "string" || value.length > MAX_PATH_LENGTH) {
    throw new Error("invalid-path");
  }
  // 空字符串表示根目录本身。
  if (value === "") {
    return "";
  }
  if (path.isAbsolute(value) || value.includes("\0")) {
    throw new Error("invalid-path");
  }
  return value;
}

function rootOf(rootId) {
  const id = assertString(rootId, "root-id");
  const root = roots.get(id);
  if (!root) {
    throw new Error("root-not-found");
  }
  return root;
}

function isWithin(rootRealPath, target) {
  const rel = path.relative(rootRealPath, target);
  return rel === "" || (!rel.startsWith("..") && !path.isAbsolute(rel));
}

/**
 * 解析根目录内已存在的路径，同时校验词法路径及其 realpath 都位于授权子树内
 * （防止符号链接逃逸）。
 */
function resolveExisting(root, relPath) {
  const relative = assertRelativePath(relPath);
  const abs = path.resolve(root.realPath, relative || ".");
  if (!isWithin(root.realPath, abs)) {
    throw new Error("path-outside-root");
  }

  let real;
  try {
    real = fs.realpathSync(abs);
  } catch {
    throw new Error("path-not-found");
  }

  if (!isWithin(root.realPath, real)) {
    throw new Error("path-outside-root");
  }
  return real;
}

/**
 * 解析即将创建或覆盖的路径。目标本身可能尚不存在，因此先对其父目录做
 * realpath 校验；若目标已存在，则直接校验其 realpath。
 */
function resolveForWrite(root, relPath) {
  const relative = assertRelativePath(relPath);
  if (!relative) {
    throw new Error("invalid-path");
  }

  const abs = path.resolve(root.realPath, relative);
  if (!isWithin(root.realPath, abs)) {
    throw new Error("path-outside-root");
  }

  if (fs.existsSync(abs)) {
    const real = fs.realpathSync(abs);
    if (!isWithin(root.realPath, real)) {
      throw new Error("path-outside-root");
    }
    return real;
  }

  let realParent;
  try {
    realParent = fs.realpathSync(path.dirname(abs));
  } catch {
    throw new Error("parent-not-found");
  }
  if (!isWithin(root.realPath, realParent)) {
    throw new Error("path-outside-root");
  }
  return abs;
}

function isValidName(name) {
  if (typeof name !== "string" || name.length === 0 || name.length > MAX_PATH_LENGTH) {
    return false;
  }
  if (name === "." || name === "..") {
    return false;
  }
  if (name.includes("/") || name.includes("\\") || name.includes("\0")) {
    return false;
  }
  return true;
}

function isBinaryBuffer(buffer) {
  const length = Math.min(buffer.length, BINARY_SNIFF_BYTES);
  for (let index = 0; index < length; index += 1) {
    if (buffer[index] === 0) {
      return true;
    }
  }
  return false;
}

async function pickRoot(browserWindow) {
  const { dialog } = require("electron");

  const options = {
    title: "Select a folder",
    properties: ["openDirectory", "createDirectory"],
  };

  const result = browserWindow
    ? await dialog.showOpenDialog(browserWindow, options)
    : await dialog.showOpenDialog(options);

  if (result.canceled || !result.filePaths || result.filePaths.length === 0) {
    return null;
  }

  const selected = result.filePaths[0];
  const realRoot = fs.realpathSync(selected);
  if (!fs.statSync(realRoot).isDirectory()) {
    throw new Error("not-a-directory");
  }

  const rootId = crypto.randomUUID();
  roots.set(rootId, { realPath: realRoot, name: path.basename(realRoot) });
  return { rootId, name: path.basename(realRoot), path: realRoot };
}

function listEntries(root, relPath) {
  const dir = resolveExisting(root, relPath);
  const stat = fs.lstatSync(dir);
  if (!stat.isDirectory()) {
    throw new Error("not-a-directory");
  }

  const names = fs.readdirSync(dir);
  const entries = [];
  for (const name of names) {
    const full = path.join(dir, name);
    let entryStat;
    try {
      entryStat = fs.lstatSync(full);
    } catch {
      continue;
    }
    entries.push({
      name,
      isDir: entryStat.isDirectory(),
      isFile: entryStat.isFile(),
      isSymlink: entryStat.isSymbolicLink(),
      size: entryStat.isFile() ? entryStat.size : 0,
    });
  }

  entries.sort((left, right) => {
    if (left.isDir !== right.isDir) {
      return left.isDir ? -1 : 1;
    }
    return left.name.localeCompare(right.name);
  });

  return { entries };
}

function readFile(root, relPath) {
  const file = resolveExisting(root, relPath);
  const stat = fs.statSync(file);
  if (stat.isDirectory()) {
    throw new Error("is-directory");
  }

  if (stat.size > MAX_READ_BYTES) {
    return { content: "", truncated: true, binary: false, size: stat.size };
  }

  const buffer = fs.readFileSync(file);
  const binary = isBinaryBuffer(buffer);
  return {
    content: binary ? "" : buffer.toString("utf8"),
    truncated: false,
    binary,
    size: stat.size,
  };
}

function writeFile(root, relPath, content) {
  if (typeof content !== "string") {
    throw new Error("invalid-content");
  }
  if (Buffer.byteLength(content, "utf8") > MAX_WRITE_BYTES) {
    throw new Error("content-too-large");
  }

  const target = resolveForWrite(root, relPath);
  fs.writeFileSync(target, content, "utf8");
  return {};
}

function createFile(root, relPath) {
  const target = resolveForWrite(root, relPath);
  if (fs.existsSync(target)) {
    throw new Error("already-exists");
  }
  fs.writeFileSync(target, "", "utf8");
  return {};
}

function createDirectory(root, relPath) {
  const target = resolveForWrite(root, relPath);
  if (fs.existsSync(target)) {
    throw new Error("already-exists");
  }
  fs.mkdirSync(target);
  return {};
}

function renameEntry(root, relPath, newName) {
  if (!isValidName(newName)) {
    throw new Error("invalid-name");
  }

  const source = resolveExisting(root, relPath);
  const target = path.join(path.dirname(source), newName);
  if (!isWithin(root.realPath, target)) {
    throw new Error("path-outside-root");
  }
  if (fs.existsSync(target)) {
    throw new Error("already-exists");
  }

  fs.renameSync(source, target);
  return {};
}

function deleteEntry(root, relPath, recursive = false) {
  const target = resolveExisting(root, relPath);
  const stat = fs.lstatSync(target);

  if (stat.isDirectory()) {
    if (!recursive && fs.readdirSync(target).length > 0) {
      throw new Error("directory-not-empty");
    }
    if (recursive) {
      fs.rmSync(target, { recursive: true, force: true });
    } else {
      fs.rmdirSync(target);
    }
  } else {
    fs.unlinkSync(target);
  }
  return {};
}

function closeRoot(rootId) {
  const id = assertString(rootId, "root-id");
  roots.delete(id);
  return {};
}

/**
 * @param {string} command
 * @param {Record<string, unknown>} payload
 * @param {import("electron").BrowserWindow | undefined} browserWindow
 */
async function handleFsCommand(command, payload = {}, browserWindow) {
  switch (command) {
    case "fs:pickRoot":
      return pickRoot(browserWindow);

    case "fs:list": {
      const root = rootOf(payload.rootId);
      return listEntries(root, payload.path);
    }

    case "fs:read": {
      const root = rootOf(payload.rootId);
      return readFile(root, payload.path);
    }

    case "fs:write": {
      const root = rootOf(payload.rootId);
      return writeFile(root, payload.path, payload.content);
    }

    case "fs:createFile": {
      const root = rootOf(payload.rootId);
      return createFile(root, payload.path);
    }

    case "fs:createDirectory": {
      const root = rootOf(payload.rootId);
      return createDirectory(root, payload.path);
    }

    case "fs:rename": {
      const root = rootOf(payload.rootId);
      return renameEntry(root, payload.path, payload.newName);
    }

    case "fs:delete": {
      const root = rootOf(payload.rootId);
      return deleteEntry(root, payload.path, Boolean(payload.recursive));
    }

    case "fs:closeRoot": {
      return closeRoot(payload.rootId);
    }

    default:
      throw new Error(`unknown-fs-command:${command}`);
  }
}

module.exports = {
  handleFsCommand,
  // 暴露给单元测试使用。
  _internals: {
    MAX_READ_BYTES,
    MAX_WRITE_BYTES,
    isBinaryBuffer,
    isWithin,
    roots,
  },
};
