import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronDown,
  FilePlus2,
  FolderOpen,
  FolderPlus,
  Save,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/confirm-dialog";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useToast } from "@/components/ui/toast";

import { electronFs, fsErrorCode } from "@/shared/fs/electronFs";
import { isBinaryExtension } from "@/shared/fs/binaryGuard";
import type { FsEditor, FsEntry } from "@/shared/fs/types";
import { IntelliJIdeaIcon } from "@/shared/icons/IntelliJIdeaIcon";
import { VscodeIcon } from "@/shared/icons/VscodeIcon";

import { CodeEditor } from "./CodeEditor";
import { FileTree, type FileTreeAction } from "./FileTree";
import { useFileTabCoordinator } from "./FileTabCoordinator";
import { baseName, joinPath, parentPath, useFsTree } from "./useFsTree";

interface FsFileMeta {
  binary: boolean;
  truncated: boolean;
}

interface NameDialogState {
  mode: "createFile" | "createDirectory" | "rename";
  parentDir: string;
  targetPath?: string;
  initialValue: string;
}

interface DeleteTargetState {
  path: string;
  entry: FsEntry;
}

export function FilePanel({ tabKey }: { tabKey?: string }) {
  const { t } = useTranslation("common");
  const toast = useToast();
  const fsTree = useFsTree();
  const coordinator = useFileTabCoordinator();
  const {
    root,
    tree,
    expanded,
    loading,
    selectedPath,
    pickRoot,
    toggleDir,
    selectEntry,
    refreshDir,
    reset: resetFsTree,
  } = fsTree;

  const [activePath, setActivePath] = useState<string | null>(null);
  const [dirtyPaths, setDirtyPaths] = useState<Set<string>>(new Set());
  const [fileMeta, setFileMeta] = useState<Map<string, FsFileMeta>>(new Map());
  const [nameDialog, setNameDialog] = useState<NameDialogState | null>(null);
  const [nameValue, setNameValue] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<DeleteTargetState | null>(null);

  const openContentsRef = useRef<Map<string, string>>(new Map());
  const currentContentsRef = useRef<Map<string, string>>(new Map());
  const savedContentRef = useRef<Map<string, string>>(new Map());
  const contentRef = useRef<string>("");

  const isDesktop = electronFs.isAvailable();

  const coordinatorRef = useRef(coordinator);
  useEffect(() => {
    coordinatorRef.current = coordinator;
  }, [coordinator]);

  useEffect(() => {
    return () => {
      const current = coordinatorRef.current;
      if (current && tabKey) {
        current.setActiveFile(tabKey, null);
        current.setActiveRoot(tabKey, null);
      }
    };
  }, [tabKey]);

  const showFsError = useCallback((error: unknown) => {
    const code = fsErrorCode(error);
    const message = t(`files.errors.${code}`, { defaultValue: t("files.errors.fs-unknown-error") });
    toast.error(message);
  }, [t, toast]);

  const openFile = useCallback(async (path: string, rootIdOverride?: string) => {
    const rootId = rootIdOverride ?? root?.rootId;
    if (!rootId) return;

    const fileKey = `${rootId}\u0000${path}`;

    if (coordinator && tabKey) {
      const existingTab = coordinator.findFileTab(fileKey, tabKey);
      if (existingTab) {
        coordinator.focusTab(existingTab);
        return;
      }
    }

    selectEntry(path);
    if (activePath === path) return;

    setActivePath(path);
    contentRef.current = currentContentsRef.current.get(path)
      ?? openContentsRef.current.get(path)
      ?? "";

    if (coordinator && tabKey) {
      coordinator.setActiveFile(tabKey, fileKey);
    }

    if (openContentsRef.current.has(path)) return;

    try {
      const result = await electronFs.read(rootId, path);
      openContentsRef.current.set(path, result.content);
      savedContentRef.current.set(path, result.content);
      contentRef.current = result.content;
      setFileMeta((previous) => new Map(previous).set(path, {
        binary: result.binary,
        truncated: result.truncated,
      }));
    } catch (error) {
      showFsError(error);
      setActivePath(null);
      if (coordinator && tabKey) {
        coordinator.setActiveFile(tabKey, null);
      }
    }
  }, [activePath, coordinator, root?.rootId, selectEntry, showFsError, tabKey]);

  const handleSelect = useCallback((path: string, entry: FsEntry) => {
    selectEntry(path);
    if (entry.isFile) {
      void openFile(path);
    }
  }, [openFile, selectEntry]);

  const handleChange = useCallback((value: string) => {
    contentRef.current = value;
    if (!activePath) return;

    currentContentsRef.current.set(activePath, value);
    const saved = savedContentRef.current.get(activePath) ?? "";
    setDirtyPaths((previous) => {
      const next = new Set(previous);
      if (value !== saved) next.add(activePath);
      else next.delete(activePath);
      return next;
    });
  }, [activePath]);

  const handleSave = useCallback(async () => {
    if (!root || !activePath) return;
    const content = contentRef.current;

    try {
      await electronFs.write(root.rootId, activePath, content);
      savedContentRef.current.set(activePath, content);
      setDirtyPaths((previous) => {
        const next = new Set(previous);
        next.delete(activePath);
        return next;
      });
      toast.success(t("files.saved"));
    } catch (error) {
      showFsError(error);
    }
  }, [activePath, root, showFsError, t, toast]);

  const closeActiveFile = useCallback(() => {
    if (!activePath) return;
    openContentsRef.current.delete(activePath);
    currentContentsRef.current.delete(activePath);
    savedContentRef.current.delete(activePath);
    contentRef.current = "";
    setFileMeta((previous) => {
      const next = new Map(previous);
      next.delete(activePath);
      return next;
    });
    setActivePath(null);
    if (coordinator && tabKey) {
      coordinator.setActiveFile(tabKey, null);
    }
  }, [activePath, coordinator, tabKey]);

  const handlePick = useCallback(async () => {
    const previousRootId = root?.rootId;
    try {
      const info = await pickRoot();
      if (!info) return;

      // 检查是否有其它文件 tab 已经打开了同一个根目录。
      if (coordinator && tabKey) {
        const existingTab = coordinator.findRootTab(info.path, tabKey);
        if (existingTab) {
          // 关闭刚刚打开的根会话，跳转到已有的 tab。
          try {
            await electronFs.closeRoot(info.rootId);
          } catch {
            // 忽略清理错误。
          }
          if (previousRootId) {
            try {
              await electronFs.closeRoot(previousRootId);
            } catch {
              // 旧会话已失效时忽略清理错误。
            }
          }
          // 清空当前 tab 的本地状态，因为旧根已关闭。
          openContentsRef.current.clear();
          currentContentsRef.current.clear();
          savedContentRef.current.clear();
          contentRef.current = "";
          setFileMeta(new Map());
          setActivePath(null);
          setDirtyPaths(new Set());
          resetFsTree();
          coordinator.setActiveFile(tabKey, null);
          coordinator.setActiveRoot(tabKey, null);
          coordinator.focusTab(existingTab);
          return;
        }
      }

      if (previousRootId) {
        try {
          await electronFs.closeRoot(previousRootId);
        } catch {
          // 旧会话已失效时忽略清理错误。
        }
      }

      openContentsRef.current.clear();
      currentContentsRef.current.clear();
      savedContentRef.current.clear();
      contentRef.current = "";
      setFileMeta(new Map());
      setActivePath(null);
      setDirtyPaths(new Set());
      if (coordinator && tabKey) {
        coordinator.setActiveFile(tabKey, null);
        coordinator.setActiveRoot(tabKey, info.path);
      }
    } catch (error) {
      showFsError(error);
    }
  }, [coordinator, pickRoot, resetFsTree, root?.rootId, showFsError, tabKey]);

  const openInEditor = useCallback(async (editor: FsEditor) => {
    if (!root) {
      toast.error(t("files.noFolderSelected"));
      return;
    }

    try {
      await electronFs.openInEditor(root.rootId, editor);
    } catch (error) {
      showFsError(error);
    }
  }, [root, showFsError, t, toast]);

  const submitName = useCallback(async () => {
    if (!root || !nameDialog) return;

    const name = nameValue.trim();
    if (!name || name === "." || name === ".." || name.includes("/") || name.includes("\\")) {
      toast.error(t("files.errors.invalid-name"));
      return;
    }

    try {
      if (nameDialog.mode === "rename" && nameDialog.targetPath) {
        const oldPath = nameDialog.targetPath;
        await electronFs.rename(root.rootId, oldPath, name);
        await refreshDir(parentPath(oldPath));
        if (activePath === oldPath || activePath?.startsWith(`${oldPath}/`)) {
          closeActiveFile();
        }
      } else {
        const targetPath = joinPath(nameDialog.parentDir, name);
        if (nameDialog.mode === "createFile") {
          await electronFs.createFile(root.rootId, targetPath);
        } else {
          await electronFs.createDirectory(root.rootId, targetPath);
        }
        await refreshDir(nameDialog.parentDir);
      }
      setNameDialog(null);
    } catch (error) {
      showFsError(error);
    }
  }, [activePath, closeActiveFile, nameDialog, nameValue, refreshDir, root, showFsError, t, toast]);

  const confirmDelete = useCallback(async () => {
    if (!root || !deleteTarget) return;

    try {
      await electronFs.remove(root.rootId, deleteTarget.path, deleteTarget.entry.isDir);
      await refreshDir(parentPath(deleteTarget.path));
      if (activePath === deleteTarget.path || activePath?.startsWith(`${deleteTarget.path}/`)) {
        closeActiveFile();
      }
    } catch (error) {
      showFsError(error);
    } finally {
      setDeleteTarget(null);
    }
  }, [activePath, closeActiveFile, deleteTarget, refreshDir, root, showFsError]);

  const handleAction = useCallback((action: FileTreeAction, path: string, entry: FsEntry) => {
    if (action === "createFile" || action === "createDirectory") {
      setNameDialog({ mode: action, parentDir: path, initialValue: "" });
      setNameValue("");
    } else if (action === "rename") {
      setNameDialog({
        mode: "rename",
        parentDir: parentPath(path),
        targetPath: path,
        initialValue: entry.name,
      });
      setNameValue(entry.name);
    } else if (action === "delete") {
      setDeleteTarget({ path, entry });
    }
  }, []);

  const activeMeta = activePath ? fileMeta.get(activePath) : undefined;
  const isDirty = activePath ? dirtyPaths.has(activePath) : false;
  const isBinary = activePath
    ? (activeMeta?.binary ?? false) || isBinaryExtension(activePath)
    : false;
  const readOnly = activeMeta?.truncated ?? false;

  if (!root) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 px-6 text-sm">
        {isDesktop ? (
          <>
            <p className="text-[var(--color-on-surface-variant)]">{t("files.emptyDescription")}</p>
            <div className="flex flex-col gap-2">
              <Button variant="outline" type="button" onClick={() => void handlePick()}>
                <FolderOpen aria-hidden="true" />
                {t("files.openFolder")}
              </Button>
            </div>
          </>
        ) : (
          <p className="text-[var(--color-on-surface-variant)]">{t("files.notDesktop")}</p>
        )}
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-between gap-2 border-b border-[var(--color-outline-variant)] px-3 py-2">
        <span className="min-w-0 truncate text-sm font-medium text-[var(--color-on-surface)]">
          {root.name}
        </span>
        <div className="flex shrink-0 items-center gap-0.5">
          <Button
            size="icon-sm"
            variant="ghost"
            type="button"
            title={t("files.openFolder")}
            aria-label={t("files.openFolder")}
            onClick={() => void handlePick()}
          >
            <FolderOpen size={14} aria-hidden="true" />
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  size="icon-sm"
                  variant="ghost"
                  type="button"
                  title={t("files.openOptions")}
                  aria-label={t("files.openOptions")}
                >
                  <ChevronDown size={13} aria-hidden="true" />
                </Button>
              }
            />
            <DropdownMenuContent align="end" side="bottom" sideOffset={4} className="w-56">
              <DropdownMenuItem onClick={() => void handlePick()}>
                <FolderOpen size={14} aria-hidden="true" />
                <span>{t("files.openFolder")}</span>
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                disabled={!root}
                title={t("files.noFolderSelected")}
                onClick={() => void openInEditor("vscode")}
              >
                <VscodeIcon size={14} className="text-muted-foreground" aria-hidden="true" />
                <span>{t("files.openInVscode")}</span>
              </DropdownMenuItem>
              <DropdownMenuItem
                disabled={!root}
                title={t("files.noFolderSelected")}
                onClick={() => void openInEditor("idea")}
              >
                <IntelliJIdeaIcon size={14} className="text-muted-foreground" aria-hidden="true" />
                <span>{t("files.openInIdea")}</span>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      <ResizablePanelGroup orientation="horizontal" className="min-h-0 flex-1">
        <ResizablePanel defaultSize="280px" minSize="20px" className="flex min-w-0 flex-col">
          <div className="flex items-center gap-1 border-b border-[var(--color-outline-variant)] px-2 py-1.5">
            <Button
              size="icon-sm"
              variant="ghost"
              type="button"
              aria-label={t("files.createFile")}
              onClick={() => handleAction("createFile", "", { name: "", isDir: false, isFile: true, isSymlink: false, size: 0 })}
            >
              <FilePlus2 size={14} aria-hidden="true" />
            </Button>
            <Button
              size="icon-sm"
              variant="ghost"
              type="button"
              aria-label={t("files.createDirectory")}
              onClick={() => handleAction("createDirectory", "", { name: "", isDir: true, isFile: false, isSymlink: false, size: 0 })}
            >
              <FolderPlus size={14} aria-hidden="true" />
            </Button>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto">
            <FileTree
              tree={tree}
              expanded={expanded}
              loading={loading}
              selectedPath={selectedPath}
              onToggleDir={toggleDir}
              onSelect={handleSelect}
              onAction={handleAction}
            />
          </div>
        </ResizablePanel>

        <ResizableHandle className="bg-[var(--color-outline-variant)]" />

        <ResizablePanel className="flex min-w-0 flex-col">
          <div className="flex items-center justify-between gap-2 border-b border-[var(--color-outline-variant)] px-3 py-1.5">
            <div className="flex min-w-0 items-center gap-2">
              {isDirty ? (
                <span
                  className="size-2 shrink-0 rounded-full bg-[var(--color-warning)]"
                  aria-label="unsaved"
                />
              ) : null}
              <span className="min-w-0 truncate text-sm text-[var(--color-on-surface)]">
                {activePath ? baseName(activePath) : t("files.selectFile")}
              </span>
            </div>
            {activePath && !isBinary ? (
              <Button
                size="sm"
                variant="outline"
                type="button"
                disabled={!isDirty || readOnly}
                onClick={() => void handleSave()}
              >
                <Save size={14} aria-hidden="true" />
                {t("files.confirm")}
              </Button>
            ) : null}
          </div>

          <div className="min-h-0 flex-1 overflow-hidden">
            {!activePath ? (
              <div className="flex h-full items-center justify-center text-sm text-[var(--color-on-surface-variant)]">
                {t("files.selectFile")}
              </div>
            ) : isBinary ? (
              <div className="flex h-full items-center justify-center text-sm text-[var(--color-on-surface-variant)]">
                {t("files.binaryFile")}
              </div>
            ) : (
              <>
                {readOnly ? (
                  <div className="border-b border-[var(--color-warning-container)] bg-[var(--color-warning-container)] px-3 py-1 text-xs text-[var(--color-on-warning-container)]">
                    {t("files.truncatedFile")}
                  </div>
                ) : null}
                <CodeEditor
                  path={activePath}
                  value={openContentsRef.current.get(activePath) ?? ""}
                  readOnly={readOnly}
                  onChange={handleChange}
                  onSave={() => void handleSave()}
                />
              </>
            )}
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>

      <Dialog open={nameDialog !== null} onOpenChange={(open) => { if (!open) setNameDialog(null); }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {nameDialog?.mode === "rename"
                ? t("files.renameTitle")
                : (nameDialog?.mode === "createFile" ? t("files.createFile") : t("files.createDirectory"))}
            </DialogTitle>
          </DialogHeader>
          <Input
            autoFocus
            value={nameValue}
            aria-label={nameDialog?.mode === "createDirectory" ? t("files.folderName") : t("files.fileName")}
            onChange={(event) => setNameValue(event.target.value)}
          />
          <DialogFooter>
            <Button variant="outline" type="button" onClick={() => setNameDialog(null)}>
              {t("files.cancel")}
            </Button>
            <Button type="button" onClick={() => void submitName()}>
              {t("files.confirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={deleteTarget !== null}
        title={t("files.deleteTitle")}
        description={t("files.deleteConfirm", { name: deleteTarget?.entry.name ?? "" })}
        confirmText={t("files.confirm")}
        cancelText={t("files.cancel")}
        tone="danger"
        onCancel={() => setDeleteTarget(null)}
        onConfirm={() => void confirmDelete()}
      />
    </div>
  );
}
