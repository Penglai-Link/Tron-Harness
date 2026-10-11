import { ChevronRight, File as FileIcon, Folder, FolderOpen } from "lucide-react";
import { memo } from "react";
import { useTranslation } from "react-i18next";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { cn } from "@/lib/utils";

import type { FsEntry } from "@/shared/fs/types";
import { joinPath } from "./useFsTree";

export type FileTreeAction = "createFile" | "createDirectory" | "rename" | "delete";

export interface FileTreeProps {
  tree: Map<string, FsEntry[]>;
  expanded: Set<string>;
  loading: Set<string>;
  selectedPath: string | null;
  onToggleDir: (path: string) => void;
  onSelect: (path: string, entry: FsEntry) => void;
  onAction: (action: FileTreeAction, path: string, entry: FsEntry) => void;
}

interface FileTreeNodeProps {
  entry: FsEntry;
  path: string;
  depth: number;
  tree: Map<string, FsEntry[]>;
  expanded: Set<string>;
  loading: Set<string>;
  selectedPath: string | null;
  onToggleDir: (path: string) => void;
  onSelect: (path: string, entry: FsEntry) => void;
  onAction: (action: FileTreeAction, path: string, entry: FsEntry) => void;
}

function FileTreeNode({
  entry,
  path,
  depth,
  tree,
  expanded,
  loading,
  selectedPath,
  onToggleDir,
  onSelect,
  onAction,
}: FileTreeNodeProps) {
  const { t } = useTranslation("common");
  const isDir = entry.isDir;
  const isExpanded = expanded.has(path);
  const isSelected = selectedPath === path;
  const children = isDir ? tree.get(path) : undefined;

  const rowClassName = cn(
    "flex min-w-0 flex-1 cursor-pointer items-center gap-1 rounded-md px-1.5 py-1 text-left text-sm transition-colors",
    isSelected
      ? "bg-[var(--color-menu-selected-surface)] text-[var(--color-on-surface)]"
      : "text-[var(--color-on-surface)] hover:bg-[var(--color-menu-hover-surface)]",
  );

  const icon = isDir
    ? (isExpanded
      ? <FolderOpen size={15} className="shrink-0 text-[var(--color-accent)]" aria-hidden="true" />
      : <Folder size={15} className="shrink-0 text-[var(--color-accent)]" aria-hidden="true" />)
    : <FileIcon size={15} className="shrink-0 text-[var(--color-on-surface-variant)]" aria-hidden="true" />;

  return (
    <div className="space-y-0.5">
      <ContextMenu>
        <ContextMenuTrigger
          render={
            <div
              className={cn(rowClassName, "w-full")}
              style={{ paddingLeft: `${depth * 12 + 6}px` }}
              role="treeitem"
              aria-selected={isSelected}
              onClick={() => onSelect(path, entry)}
            >
              {isDir ? (
                <button
                  type="button"
                  className="flex size-5 shrink-0 items-center justify-center rounded text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]"
                  aria-label={isExpanded ? t("common:actions.collapse", "Collapse") : t("common:actions.expand", "Expand")}
                  aria-expanded={isExpanded}
                  onClick={(event) => {
                    event.stopPropagation();
                    onToggleDir(path);
                  }}
                >
                  <ChevronRight
                    size={14}
                    className={cn("transition-transform", isExpanded && "rotate-90")}
                    aria-hidden="true"
                  />
                </button>
              ) : (
                <span className="block size-5 shrink-0" aria-hidden="true" />
              )}
              {icon}
              <span className="min-w-0 flex-1 truncate">{entry.name}</span>
            </div>
          }
        />
        <ContextMenuContent>
          {isDir ? (
            <>
              <ContextMenuItem onClick={() => onAction("createFile", path, entry)}>

                {t("files.createFile")}
              </ContextMenuItem>
              <ContextMenuItem onClick={() => onAction("createDirectory", path, entry)}>

                {t("files.createDirectory")}
              </ContextMenuItem>
              <ContextMenuSeparator />
            </>
          ) : null}
          <ContextMenuItem onClick={() => onAction("rename", path, entry)}>

            {t("files.rename")}
          </ContextMenuItem>
          <ContextMenuItem onClick={() => onAction("delete", path, entry)}>

            {t("files.delete")}
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>

      {isDir && isExpanded ? (
        <div className="space-y-0.5">
          {loading.has(path) && !children ? (
            <div className="py-1 pl-4 text-xs text-[var(--color-on-surface-variant)]" style={{ paddingLeft: `${(depth + 1) * 12 + 6}px` }}>
              {t("files.loading")}
            </div>
          ) : null}
          {children?.map((child) => (
            <FileTreeNode
              key={child.name}
              entry={child}
              path={joinPath(path, child.name)}
              depth={depth + 1}
              tree={tree}
              expanded={expanded}
              loading={loading}
              selectedPath={selectedPath}
              onToggleDir={onToggleDir}
              onSelect={onSelect}
              onAction={onAction}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

const MemoizedFileTreeNode = memo(FileTreeNode);

export function FileTree({
  tree,
  expanded,
  loading,
  selectedPath,
  onToggleDir,
  onSelect,
  onAction,
}: FileTreeProps) {
  const rootEntries = tree.get("") ?? [];

  return (
    <div role="tree" className="py-1" aria-label="Files">
      {rootEntries.map((entry) => (
        <MemoizedFileTreeNode
          key={entry.name}
          entry={entry}
          path={entry.name}
          depth={0}
          tree={tree}
          expanded={expanded}
          loading={loading}
          selectedPath={selectedPath}
          onToggleDir={onToggleDir}
          onSelect={onSelect}
          onAction={onAction}
        />
      ))}
    </div>
  );
}
