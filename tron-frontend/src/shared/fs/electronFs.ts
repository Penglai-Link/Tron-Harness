import { DesktopBridge } from "@/shared/desktop/DesktopBridge";

import type { FsEntry, FsReadResult, FsRootInfo } from "./types";

/**
 * 面向渲染进程的文件系统门面。
 *
 * 这是「文件」面板唯一通信的模块。它封装了通用的 `DesktopBridge`，并以类型化
 * 参数暴露更高层的 `fs:*` 操作。把它集中在这里（而不是在面板里到处散落
 * `invoke` 调用）能让面板在单元测试中拥有单一、可 mock 的接缝。
 */
export class FileSystemClient {
  private readonly bridge: DesktopBridge;

  constructor(bridge: DesktopBridge = new DesktopBridge()) {
    this.bridge = bridge;
  }

  public isAvailable(): boolean {
    return this.bridge.isDesktop();
  }

  public async pickRoot(): Promise<FsRootInfo | null> {
    return this.bridge.pickFsRoot();
  }

  public async list(rootId: string, path: string): Promise<FsEntry[]> {
    const result = await this.bridge.listFsEntries(rootId, path);
    return result.entries ?? [];
  }

  public async read(rootId: string, path: string): Promise<FsReadResult> {
    return this.bridge.readFsFile(rootId, path);
  }

  public async write(rootId: string, path: string, content: string): Promise<void> {
    return this.bridge.writeFsFile(rootId, path, content);
  }

  public async createFile(rootId: string, path: string): Promise<void> {
    return this.bridge.createFsFile(rootId, path);
  }

  public async createDirectory(rootId: string, path: string): Promise<void> {
    return this.bridge.createFsDirectory(rootId, path);
  }

  public async rename(rootId: string, path: string, newName: string): Promise<void> {
    return this.bridge.renameFsEntry(rootId, path, newName);
  }

  public async remove(rootId: string, path: string, recursive: boolean): Promise<void> {
    return this.bridge.deleteFsEntry(rootId, path, recursive);
  }

  public async closeRoot(rootId: string): Promise<void> {
    return this.bridge.closeFsRoot(rootId);
  }
}

/** 面板在运行时使用的共享实例。 */
export const electronFs = new FileSystemClient();

/**
 * 从 IPC 错误中提取稳定的机器可读错误码。特权侧 `fs-gateway.cjs` 会抛出
 * `new Error("<code>")`；Electron 随后在渲染进程中以类似
 * `"Error invoking remote method 'tron-desktop:invoke': Error: <code>"` 的形式
 * 呈现，因此这里通过匹配已知错误码来替代返回带有前缀的原始消息。
 */
const KNOWN_FS_ERROR_CODES = [
  "not-in-desktop",
  "root-not-found",
  "path-outside-root",
  "path-not-found",
  "parent-not-found",
  "not-a-directory",
  "not-a-file",
  "is-directory",
  "already-exists",
  "directory-not-empty",
  "invalid-name",
  "content-too-large",
  "invalid-root-id",
  "invalid-path",
  "invalid-content",
  "unknown-fs-command",
] as const;

export function fsErrorCode(error: unknown): string {
  const message = (() => {
    if (error instanceof Error && error.message) {
      return error.message;
    }
    if (typeof error === "string" && error.trim()) {
      return error.trim();
    }
    return "";
  })();

  if (!message) {
    return "fs-unknown-error";
  }

  for (const code of KNOWN_FS_ERROR_CODES) {
    if (message.includes(code)) {
      return code;
    }
  }

  return "fs-unknown-error";
}
