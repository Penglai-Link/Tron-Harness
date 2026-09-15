/**
 * 文件面板的共享类型。
 *
 * 这些类型描述 Electron `fs:*` 命令面向渲染进程的契约。特权实现位于
 * `electron/main/fs-gateway.cjs`；渲染进程只会看到这些可序列化的普通结构。
 */

/** `fs:list` 返回的单个目录项。 */
export interface FsEntry {
  name: string;
  isDir: boolean;
  isFile: boolean;
  isSymlink: boolean;
  size: number;
}

/** `fs:list` 的返回结果。 */
export interface FsListResult {
  entries: FsEntry[];
}

/** `fs:read` 的返回结果。 */
export interface FsReadResult {
  content: string;
  truncated: boolean;
  binary: boolean;
  size: number;
}

/** `fs:pickRoot` 的返回结果。 */
export interface FsRootInfo {
  rootId: string;
  name: string;
  path: string;
}

/** 可通过 `fs:openInEditor` 启动的外部编辑器。 */
export type FsEditor = "vscode" | "idea";
