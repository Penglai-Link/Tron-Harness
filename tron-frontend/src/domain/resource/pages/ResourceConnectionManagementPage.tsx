import { MoreHorizontal, RefreshCcw } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { Button } from "@/components/ui/button";
import {
  DataTable as TanStackDataTable,
  TablePagination as TanStackTablePagination,
  type TableColumnDef,
  useServerTable,
} from "@/components/table";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

import { toastManager } from "@/components/ui/toast";
import {
  DataSourceCreateDialog,
  ServerAccountCreateDialog,
  ServerCreateDialog,
} from "@/domain/resource/components";
import { deleteConnection } from "@/domain/resource/api";
import { deleteMockResource } from "@/domain/resource/mock";
import type { ResourceKind, ResourceRecord } from "@/domain/resource/model";
import {
  queryResourcePage,
  queryResourceConnectionPage,
  useManagedResourceRecords,
} from "@/domain/resource/hooks";
import { SearchField } from "@/components/search-field";
import {
  PageHeader,
  useLocaleFormatters,
  useWorkspaceSpaces,
} from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";

type ResourceConnectionKind = "dataSource" | "server" | "serverAccount";

interface ResourceConnectionManagementPageProps {
  kind: ResourceConnectionKind;
}

const PAGE_SIZE = 15;

const KIND_CONFIG: Record<
  ResourceConnectionKind,
  {
    basePath: string;
    kinds: readonly ResourceKind[];
  }
> = {
  dataSource: {
    basePath: "/resource-management/data-sources",
    kinds: [
      "MYSQL_CONNECTION",
      "POSTGRESQL_CONNECTION",
      "REDIS_CONNECTION",
      "KAFKA_CONNECTION",
    ],
  },
  server: {
    basePath: "/resource-management/servers",
    kinds: ["SERVER_CONNECTION"],
  },
  serverAccount: {
    basePath: "/resource-management/server-accounts",
    kinds: ["SERVER_ACCOUNT"],
  },
};

const DATA_SOURCE_KIND_LABELS: Record<string, string> = {
  KAFKA_CONNECTION: "Kafka",
  MYSQL_CONNECTION: "MySQL",
  POSTGRESQL_CONNECTION: "PostgreSQL",
  REDIS_CONNECTION: "Redis",
};

function firstEndpoint(attributes: Record<string, unknown>) {
  const values = Array.isArray(attributes.endpoints)
    ? attributes.endpoints
    : Array.isArray(attributes.brokers)
    ? attributes.brokers
    : [];
  const endpoint = values[0];
  if (!endpoint || typeof endpoint !== "object") return null;
  const { host, port } = endpoint as Record<string, unknown>;
  return typeof host === "string" && typeof port === "number"
    ? `${host}:${port}`
    : null;
}

function stringAttribute(record: ResourceRecord, key: string) {
  const value = record.attributes?.[key];
  return typeof value === "string" ? value : "—";
}

export function ResourceConnectionManagementPage({
  kind,
}: ResourceConnectionManagementPageProps) {
  const { t } = useTranslation("pages");
  const navigate = useNavigate();
  const { formatDateTime } = useLocaleFormatters();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const workspaceCode = workspace?.workspaceCode ?? "";
  const workspaceName = workspace?.name ?? "";
  const config = KIND_CONFIG[kind];
  const [search, setSearch] = useState("");
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<ResourceRecord | null>(null);
  const [deleting, setDeleting] = useState(false);
  const resourceQueryParams = useMemo(
    () => ({
      context: "ALL" as const,
      kinds: config.kinds,
      search,
      workspaceCode: workspaceCode || null,
      workspaceName,
    }),
    [config.kinds, search, workspaceCode, workspaceName],
  );
  const serversQuery = useManagedResourceRecords({
    kinds: ["SERVER_CONNECTION"],
    search: "",
    workspaceCode: workspaceCode || null,
    workspaceName,
  });
  const pageT = (key: string, options?: Record<string, unknown>) =>
    t(`resourceConnections.${key}`, options);

  function connectionLabel(record: ResourceRecord) {
    if (kind === "server") {
      const host = stringAttribute(record, "host");
      const port = record.attributes?.port;
      return `${host}:${typeof port === "number" ? port : "—"}`;
    }
    if (kind === "serverAccount") {
      return `${stringAttribute(record, "username")} @ ${stringAttribute(
        record,
        "serverCode",
      )}`;
    }
    return firstEndpoint(record.attributes ?? {}) ?? "—";
  }

  function secondaryLabel(record: ResourceRecord) {
    if (kind === "serverAccount") {
      return pageT(
        `authentication.${
          stringAttribute(record, "authenticationType") === "PrivateKey"
            ? "privateKey"
            : "password"
        }`,
      );
    }
    if (kind === "dataSource") {
      return DATA_SOURCE_KIND_LABELS[record.kind] ?? record.kind;
    }
    return "SSH";
  }

  const columns = useMemo<TableColumnDef<ResourceRecord>[]>(
    () => [
      {
        accessorKey: "name",
        cell: ({ row }) => (
          <div className="min-w-0">
            <p
              className="truncate font-medium text-[var(--color-primary)]"
              title={row.original.name}
            >
              {row.original.name}
            </p>
            <p
              className="mt-1 truncate font-mono text-[12px] text-[var(--color-on-surface-variant)]"
              title={row.original.resourceCode}
            >
              {row.original.resourceCode}
            </p>
          </div>
        ),
        header: pageT("columns.name"),
        id: "resource",
        meta: { width: "30%" },
      },
      {
        cell: ({ row }) => (
          <span className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">
            {connectionLabel(row.original)}
          </span>
        ),
        header: pageT(
          kind === "serverAccount"
            ? "columns.accountBinding"
            : "columns.endpoint",
        ),
        id: "endpoint",
      },
      {
        cell: ({ row }) => (
          <Badge variant="outline">{secondaryLabel(row.original)}</Badge>
        ),
        header: pageT(
          kind === "dataSource"
            ? "columns.type"
            : kind === "serverAccount"
            ? "columns.authentication"
            : "columns.protocol",
        ),
        id: "type",
      },
      {
        cell: ({ row }) => (
          <Badge
            variant={
              row.original.sourceKind === "LOCAL"
                ? "success"
                : row.original.sourceKind === "INHERITED"
                ? "info"
                : row.original.sourceKind === "OVERRIDE"
                ? "warning"
                : "outline"
            }
          >
            {pageT(`source.${row.original.sourceKind.toLowerCase()}`)}
          </Badge>
        ),
        header: pageT("columns.source"),
        id: "source",
      },
      {
        cell: ({ row }) => (
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">
            {formatDateTime(row.original.updatedAt, {
              dateStyle: "medium",
              timeStyle: "short",
            })}
          </span>
        ),
        header: pageT("columns.updatedAt"),
        id: "updatedAt",
      },
      {
        cell: ({ row }) =>
          row.original.sourceKind !== "INHERITED" ? (
            <DropdownMenu>
              <DropdownMenuTrigger
                render={(
                  <Button
                    aria-label={pageT("columns.actions")}
                    size="icon"
                    variant="ghost"
                    onClick={(event) => event.stopPropagation()}
                    onKeyDown={(event) => event.stopPropagation()}
                  >
                    <MoreHorizontal aria-hidden="true" size={16} />
                  </Button>
                )}
              />
              <DropdownMenuContent align="end">
                <DropdownMenuItem
                  onClick={(event) => {
                    event.stopPropagation();
                    navigate(
                      `${
                        config.basePath
                      }/detail?resourceCode=${encodeURIComponent(
                        row.original.resourceCode,
                      )}`,
                    );
                  }}
                >
                  {pageT("actions.edit")}
                </DropdownMenuItem>
                <DropdownMenuItem
                  variant="destructive"
                  onClick={(event) => {
                    event.stopPropagation();
                    setDeleteTarget(row.original);
                  }}
                >

                  {pageT("actions.delete")}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          ) : null,
        header: pageT("columns.actions"),
        id: "actions",
        meta: { align: "right" },
        size: 112,
      },
    ],
    [config.basePath, formatDateTime, kind, navigate, t],
  );

  const resourceTable = useServerTable({
    columns,
    enabled: Boolean(workspaceCode),
    getRowId: (row) => `${row.workspaceCode}:${row.resourceCode}`,
    initialPageSize: PAGE_SIZE,
    params: resourceQueryParams,
    queryFn: ({ pageIndex, pageSize, params }) => {
      const query = {
        ...params,
        current: pageIndex + 1,
        size: pageSize,
      };
      return getServiceDataSource("resource") === "api"
        ? queryResourceConnectionPage(query)
        : queryResourcePage(query);
    },
    resource: `resource-connections:${kind}`,
  });

  async function confirmDelete() {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      if (getServiceDataSource("resource") === "api") {
        const result = await deleteConnection({
          resourceCode: deleteTarget.resourceCode,
          workspaceCode,
        });
        if (!result.deleted) throw new Error(pageT("messages.deleteFailed"));
      } else if (
        !deleteMockResource(workspaceCode, deleteTarget.resourceCode)
      ) {
        throw new Error(pageT("messages.deleteFailed"));
      }
      toastManager.add({
        description: pageT("messages.deleted", { name: deleteTarget.name }),
        title: pageT("messages.deletedTitle"),
        type: "success",
      });
      setDeleteTarget(null);
      await resourceTable.refresh();
    } catch (cause) {
      toastManager.add({
        description:
          cause instanceof Error
            ? cause.message
            : pageT("messages.deleteFailed"),
        title: pageT("messages.deleteFailed"),
        type: "error",
      });
    } finally {
      setDeleting(false);
    }
  }

  const title = pageT(`${kind}.title`);

  return (
    <>
      <PageHeader
        title={title}
        actions={(
          <>
            <Button
              aria-label={pageT("actions.refresh")}
              size="lg"
              variant="outline"
              onClick={() => {
                void resourceTable.refresh();
              }}
            >
              <RefreshCcw aria-hidden="true" size={15} />
            </Button>
            <Button
              disabled={!workspace}
              size="lg"
              onClick={() => setCreateOpen(true)}
            >

              {pageT(`${kind}.createAction`)}
            </Button>
          </>
        )}
      />
      <div className="mt-5">
        <TanStackDataTable<ResourceRecord>
          variant="default"
          emptyState={
            workspace ? pageT(`${kind}.empty`) : pageT("workspaceRequired")
          }
          toolbar={(
            <SearchField
              ariaLabel={pageT("search.label")}
              className="w-full sm:w-72"
              placeholder={pageT(`${kind}.searchPlaceholder`)}
              value={search}
              onChange={setSearch}
            />
          )}
          error={resourceTable.error}
          loading={resourceTable.isLoading}
          pagination={
            <TanStackTablePagination
              itemLabel={pageT("pagination.itemLabel", {
                defaultValue: "records",
              })}
              pageSizeOptions={[15, 25, 50]}
              table={resourceTable.table}
              totalItems={resourceTable.data.total}
            />
          }
          table={resourceTable.table}
          tableClassName="min-w-[900px]"
          onRowClick={(row) =>
            navigate(
              `${config.basePath}/detail?resourceCode=${encodeURIComponent(
                row.resourceCode,
              )}`,
            )
          }
        />
      </div>

      {kind === "dataSource" ? (
        <DataSourceCreateDialog
          onCreated={resourceTable.refresh}
          onOpenChange={setCreateOpen}
          open={createOpen}
          workspaceCode={workspaceCode}
        />
      ) : kind === "server" ? (
        <ServerCreateDialog
          onCreated={() => {
            void resourceTable.refresh();
            serversQuery.refresh();
          }}
          onOpenChange={setCreateOpen}
          open={createOpen}
          workspaceCode={workspaceCode}
        />
      ) : (
        <ServerAccountCreateDialog
          onCreated={resourceTable.refresh}
          onOpenChange={setCreateOpen}
          open={createOpen}
          servers={serversQuery.data}
          workspaceCode={workspaceCode}
        />
      )}

      <Dialog
        onOpenChange={(open) => !open && setDeleteTarget(null)}
        open={Boolean(deleteTarget)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {pageT("delete.title", { name: deleteTarget?.name ?? "" })}
            </DialogTitle>
          </DialogHeader>
          <DialogFooter>
            <Button
              disabled={deleting}
              variant="outline"
              onClick={() => setDeleteTarget(null)}
            >
              {pageT("actions.cancel")}
            </Button>
            <Button
              disabled={deleting}
              variant="destructive"
              onClick={() => {
                void confirmDelete();
              }}
            >

              {pageT("actions.delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export function DataSourceManagementPage() {
  return <ResourceConnectionManagementPage kind="dataSource" />;
}

export function ServerManagementPage() {
  return <ResourceConnectionManagementPage kind="server" />;
}

export function ServerAccountManagementPage() {
  return <ResourceConnectionManagementPage kind="serverAccount" />;
}
