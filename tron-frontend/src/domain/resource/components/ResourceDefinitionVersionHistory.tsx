import { useTranslation } from "react-i18next";
import { MoreHorizontal } from "lucide-react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { buttonVariants } from "@/components/ui/button";
import type { ResourceDefinitionVersionDetail } from "@/domain/resource/model";
import {
  Badge,
  DataTable,
  TablePagination,
  useLocaleFormatters,
  useTablePagination,
} from "@/shared";

interface ResourceDefinitionVersionHistoryProps {
  onAction: (record: ResourceDefinitionVersionDetail, action: "activate" | "createDraft" | "discard" | "edit" | "open" | "publish") => void;
  onSelect: (record: ResourceDefinitionVersionDetail) => void;
  selectedVersion: string;
  versions: ResourceDefinitionVersionDetail[];
}

export function ResourceDefinitionVersionHistory({ onAction, onSelect, selectedVersion, versions }: ResourceDefinitionVersionHistoryProps) {
  const { t } = useTranslation("pages");
  const { formatDateTime } = useLocaleFormatters();
  const pagination = useTablePagination(versions);
  const historyT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`resourceDefinitionEditor.history.${key}`, { defaultValue, ...options })
  );
  const formatTimestamp = (value: string | null) => value
    ? formatDateTime(value, { dateStyle: "medium", timeStyle: "short" })
    : historyT("never", "Not published");

  function lifecycleTone(status: ResourceDefinitionVersionDetail["lifecycleStatus"]) {
    if (status === "ACTIVE") return "success" as const;
    if (status === "DRAFT") return "warning" as const;
    return "info" as const;
  }

  function renderActions(record: ResourceDefinitionVersionDetail) {
    return (
      <div className="flex justify-end" onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}>
        <DropdownMenu>
          <DropdownMenuTrigger
            aria-label={historyT("actions.openMenu", "Open actions for {{version}}", { version: record.definitionVersion })}
            className={buttonVariants({ size: "icon", variant: "ghost" })}
          >
            <MoreHorizontal aria-hidden="true" size={16} />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-44" onClick={(event) => event.stopPropagation()}>
            {record.lifecycleStatus === "DRAFT" ? (
              <>
                <DropdownMenuItem onClick={() => onAction(record, "edit")}>{historyT("actions.edit", "Edit draft")}</DropdownMenuItem>
                <DropdownMenuItem onClick={() => onAction(record, "publish")}>{historyT("actions.publish", "Publish")}</DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem variant="destructive" onClick={() => onAction(record, "discard")}>{historyT("actions.discard", "Discard draft")}</DropdownMenuItem>
              </>
            ) : (
              <>
                <DropdownMenuItem onClick={() => onAction(record, "open")}>{historyT("actions.open", "View version")}</DropdownMenuItem>
                <DropdownMenuItem onClick={() => onAction(record, record.lifecycleStatus === "ACTIVE" ? "createDraft" : "activate")}>
                  {record.lifecycleStatus === "ACTIVE"
                    ? historyT("actions.createDraft", "Create draft")
                    : historyT("actions.activate", "Activate version")}
                </DropdownMenuItem>
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-base font-semibold">{historyT("title", "Version history")}</h2>
        <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">
          {historyT("description", "Select a snapshot to update the entire detail page. Drafts remain editable; published snapshots are read-only.")}
        </p>
      </div>
      <DataTable<ResourceDefinitionVersionDetail>
        columns={[
          {
            id: "version",
            header: historyT("columns.version", "Version"),
            width: "20%",
            headerClassName: "max-sm:w-[54%] max-sm:px-3",
            cellClassName: "max-sm:w-[54%] max-sm:px-3",
            render: (record) => (
              <div className="min-w-0">
                <p className="font-mono text-sm font-medium">{record.definitionVersion}</p>
                <p className="mt-1 text-[11px] text-[var(--color-on-surface-variant)] sm:hidden">
                  {historyT("recent", "Recent")}: {formatTimestamp(record.lifecycleStatus === "DRAFT" ? record.updatedAt : record.publishedAt)}
                </p>
                <p className="mt-0.5 text-[11px] text-[var(--color-on-surface-variant)] sm:hidden">
                  {historyT("created", "Created")}: {formatTimestamp(record.createdAt)}
                </p>
              </div>
            ),
          },
          {
            id: "status",
            header: historyT("columns.status", "Status"),
            width: "22%",
            headerClassName: "max-sm:w-[31%] max-sm:px-2",
            cellClassName: "max-sm:w-[31%] max-sm:px-2",
            render: (record) => (
              <div className="flex flex-wrap gap-1.5">
                <Badge tone={lifecycleTone(record.lifecycleStatus)}>{record.lifecycleStatus}</Badge>
                <Badge tone={record.validationStatus === "VALID" ? "success" : "error"}>{record.validationStatus}</Badge>
              </div>
            ),
          },
          {
            id: "recent",
            header: historyT("columns.recent", "Recent activity"),
            width: "23%",
            colClassName: "hidden sm:table-column",
            headerClassName: "hidden sm:table-cell",
            cellClassName: "hidden whitespace-nowrap sm:table-cell",
            render: (record) => (
              <span className="text-[12px] text-[var(--color-on-surface-variant)]">
                {formatTimestamp(record.lifecycleStatus === "DRAFT" ? record.updatedAt : record.publishedAt)}
              </span>
            ),
          },
          {
            id: "created",
            header: historyT("columns.created", "Created at"),
            width: "25%",
            colClassName: "hidden md:table-column",
            headerClassName: "hidden md:table-cell",
            cellClassName: "hidden whitespace-nowrap md:table-cell",
            render: (record) => (
              <span className="text-[12px] text-[var(--color-on-surface-variant)]">{formatTimestamp(record.createdAt)}</span>
            ),
          },
          {
            id: "actions",
            header: historyT("columns.actions", "Actions"),
            align: "right",
            width: "10%",
            headerClassName: "whitespace-nowrap max-sm:w-[15%] max-sm:px-1",
            cellClassName: "max-sm:w-[15%] max-sm:px-1",
            render: renderActions,
          },
        ]}
        emptyState={historyT("empty", "No version snapshots are available.")}
        onRowClick={onSelect}
        pagination={<TablePagination itemLabel={historyT("itemLabel", "versions")} showRange={false} {...pagination.paginationProps} />}
        rowClassName={(record) => (record.definitionVersion === selectedVersion ? "bg-[var(--color-surface-container-low)]" : undefined)}
        rowKey={(record) => record.definitionVersion}
        rows={pagination.visibleItems}
        tableClassName="min-w-full table-fixed max-sm:w-[calc(100%-1px)] max-sm:min-w-0 max-sm:[&>colgroup]:hidden"
      />
    </div>
  );
}
