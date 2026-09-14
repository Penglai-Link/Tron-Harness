import { useCallback, useRef, useState } from "react";

import { electronFs, type FileSystemClient } from "@/shared/fs/electronFs";
import type { FsEntry, FsRootInfo } from "@/shared/fs/types";

/** 将子名称拼接到父路径上（路径统一使用 "/" 分隔）。 */
export function joinPath(parent: string, name: string): string {
  return parent ? `${parent}/${name}` : name;
}

/** 返回文件/目录路径的父目录路径。 */
export function parentPath(path: string): string {
  const index = path.lastIndexOf("/");
  return index <= 0 ? "" : path.slice(0, index);
}

/** 返回路径末尾的名称。 */
export function baseName(path: string): string {
  const index = path.lastIndexOf("/");
  return index < 0 ? path : path.slice(index + 1);
}

interface UseFsTreeOptions {
  client?: FileSystemClient;
}

export interface UseFsTreeResult {
  root: FsRootInfo | null;
  tree: Map<string, FsEntry[]>;
  expanded: Set<string>;
  loading: Set<string>;
  selectedPath: string | null;
  pickRoot: () => Promise<FsRootInfo | null>;
  toggleDir: (path: string) => void;
  selectEntry: (path: string) => void;
  refreshDir: (path: string) => Promise<void>;
  reset: () => void;
}

/**
 * 「文件」面板使用的懒加载目录树状态。
 *
 * 目录内容在展开时逐层获取，并以相对路径为键缓存在 `Map<string, FsEntry[]>`
 * 中。根目录路径为空字符串。
 */
export function useFsTree(options: UseFsTreeOptions = {}): UseFsTreeResult {
  const client = options.client ?? electronFs;

  const [root, setRoot] = useState<FsRootInfo | null>(null);
  const [tree, setTree] = useState<Map<string, FsEntry[]>>(new Map());
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState<Set<string>>(new Set());
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  const rootRef = useRef<FsRootInfo | null>(null);
  const treeRef = useRef<Map<string, FsEntry[]>>(new Map());
  const expandedRef = useRef<Set<string>>(new Set());

  const loadDir = useCallback(async (path: string) => {
    const currentRoot = rootRef.current;
    if (!currentRoot) return;

    setLoading((previous) => new Set(previous).add(path));
    try {
      const entries = await client.list(currentRoot.rootId, path);
      treeRef.current = new Map(treeRef.current).set(path, entries);
      setTree(treeRef.current);
    } finally {
      setLoading((previous) => {
        const next = new Set(previous);
        next.delete(path);
        return next;
      });
    }
  }, [client]);

  const pickRoot = useCallback(async () => {
    const info = await client.pickRoot();
    if (!info) return null;

    rootRef.current = info;
    setRoot(info);

    const rootEntries = await client.list(info.rootId, "");
    treeRef.current = new Map([["", rootEntries]]);
    setTree(treeRef.current);
    setSelectedPath(null);

    expandedRef.current = new Set();
    setExpanded(new Set());
    return info;
  }, [client]);

  const toggleDir = useCallback((path: string) => {
    if (expandedRef.current.has(path)) {
      const next = new Set(expandedRef.current);
      next.delete(path);
      expandedRef.current = next;
      setExpanded(next);
      return;
    }

    const next = new Set(expandedRef.current);
    next.add(path);
    expandedRef.current = next;
    setExpanded(next);

    if (!treeRef.current.has(path)) {
      void loadDir(path);
    }
  }, [loadDir]);

  const selectEntry = useCallback((path: string) => {
    setSelectedPath(path);
  }, []);

  const refreshDir = useCallback(async (path: string) => {
    const currentRoot = rootRef.current;
    if (!currentRoot) return;

    const entries = await client.list(currentRoot.rootId, path);
    treeRef.current = new Map(treeRef.current).set(path, entries);
    setTree(treeRef.current);
  }, [client]);

  const reset = useCallback(() => {
    rootRef.current = null;
    treeRef.current = new Map();
    expandedRef.current = new Set();
    setRoot(null);
    setTree(new Map());
    setExpanded(new Set());
    setSelectedPath(null);
  }, []);

  return {
    root,
    tree,
    expanded,
    loading,
    selectedPath,
    pickRoot,
    toggleDir,
    selectEntry,
    refreshDir,
    reset,
  };
}
