import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router";
import { RefreshCcw, Upload } from "lucide-react";
import { SelectItem } from "@/components/ui/select";

import { Button } from "@/components/ui/button";
import {
  DataTable as TanStackDataTable,
  TablePagination as TanStackTablePagination,
  type TableColumnDef,
  useServerTable,
} from "@/components/table";
import {
  QuickResourceUploadDialog,
  type QuickResourceCreateCommand,
} from "@/domain/resource/features/quick-upload";
import { createResource, fromResourceKindCode } from "@/domain/resource/api";
import { resourceFileKinds } from "@/domain/resource/constants";
import {
  queryResourcePage,
  useSettingsResourceScope,
} from "@/domain/resource/hooks";
import { createMockQuickResource } from "@/domain/resource/mock";
import type {
  QuickResourceUploadHandoffPayload,
  ResourceKind,
  ResourceRecord,
  ResourceSourceKind,
} from "@/domain/resource/model";
import { useQuickResourceUploadHandoffStore } from "@/domain/resource/model";
import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextInput,
  useLocaleFormatters,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Badge } from "@/shared/ui/Badge";

const allResourceKinds: ResourceKind[] = [
  "FILE",
  "IMAGE",
  "PDF",
  "MYSQL_CONNECTION",
  "POSTGRESQL_CONNECTION",
  "REDIS_CONNECTION",
  "KAFKA_CONNECTION",
  "SERVER_CONNECTION",
  "SERVER_ACCOUNT",
];

function formatByteSize(byteSize: number) {
  if (byteSize < 1024) return `${byteSize} B`;
  if (byteSize < 1024 * 1024) return `${(byteSize / 1024).toFixed(1)} KB`;
  return `${(byteSize / (1024 * 1024)).toFixed(1)} MB`;
}

export function ResourcesPage() {
  const { t } = useTranslation(["pages", "common"]);
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const { formatDateTime } = useLocaleFormatters();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const consoleWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const settingsScope = useSettingsResourceScope();
  const activeWorkspace = settingsScope.isSettings
    ? settingsScope.workspaceCode
      ? {
          workspaceCode: settingsScope.workspaceCode,
          name: settingsScope.workspaceName,
        }
      : undefined
    : consoleWorkspace;
  const isFileManagement = location.pathname.startsWith(
    "/resource-management/files",
  );
  const isPlatformResources = settingsScope.isSettings;
  const basePath = settingsScope.isSettings
    ? "/settings/resources"
    : isFileManagement
    ? "/resource-management/files"
    : "/resources";
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const activeWorkspaceName = activeWorkspace?.name ?? "";
  const isApi = getServiceDataSource("resource") === "api";
  const [query, setQuery] = useState("");
  const [kindFilter, setKindFilter] = useState<string>("ALL");
  const [contextFilter, setContextFilter] = useState<
    ResourceSourceKind | "ALL"
  >("ALL");
  const [quickUploadOpen, setQuickUploadOpen] = useState(false);
  const kindOptions = isFileManagement ? resourceFileKinds : allResourceKinds;
  const resourceQueryParams = useMemo(
    () => ({
      context: isPlatformResources ? "ALL" : contextFilter,
      kinds:
        kindFilter === "ALL"
          ? isFileManagement
            ? resourceFileKinds
            : []
          : [kindFilter as ResourceKind],
      search: query,
      workspaceCode: activeWorkspaceCode || null,
      workspaceName: activeWorkspaceName,
    }),
    [
      activeWorkspaceCode,
      activeWorkspaceName,
      contextFilter,
      isFileManagement,
      isPlatformResources,
      kindFilter,
      query,
    ],
  );

  const pageT = (key: string, options?: Record<string, unknown>) =>
    t(`pages:resources.${key}`, options);

  const columns = useMemo<TableColumnDef<ResourceRecord>[]>(
    () => [
      {
        accessorKey: "name",
        cell: ({ row }) => (
          <div className="min-w-0">
            <p
              className="truncate font-medium text-[var(--color-on-surface)]"
              title={row.original.name}
            >
              {row.original.name}
            </p>
            <div className="mt-1 flex min-w-0 items-center gap-2 text-[12px] text-[var(--color-on-surface-variant)]">
              <span
                className="truncate font-mono"
                title={row.original.resourceCode}
              >
                {row.original.resourceCode}
              </span>
              <span
                aria-hidden="true"
                className="hidden text-[var(--color-outline)] xl:inline"
              >
                ·
              </span>
              <span
                className="hidden truncate xl:inline"
                title={row.original.description}
              >
                {row.original.description}
              </span>
            </div>
          </div>
        ),
        header: pageT("columns.resource"),
        id: "resource",
        meta: {
          cellClassName: "px-4 py-2.5 max-sm:w-[58%] max-sm:px-3",
          headerClassName: "px-4 max-sm:w-[58%] max-sm:px-3",
          width: "32%",
        },
      },
      {
        cell: ({ row }) => (
          <Badge tone="outline">
            {pageT(`kind.${row.original.kind.toLowerCase()}`)}
          </Badge>
        ),
        header: pageT("columns.kind"),
        id: "kind",
        meta: {
          cellClassName: "hidden px-4 py-2.5 lg:table-cell",
          colClassName: "hidden lg:table-column",
          headerClassName: "hidden px-4 lg:table-cell",
          width: "17%",
        },
      },
      {
        cell: ({ row }) => (
          <span className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">
            {row.original.file
              ? formatByteSize(row.original.file.byteSize)
              : "—"}
          </span>
        ),
        header: pageT("columns.fileSize"),
        id: "fileSize",
        meta: {
          cellClassName: "hidden whitespace-nowrap px-4 py-2.5 sm:table-cell",
          colClassName: "hidden sm:table-column",
          headerClassName: "hidden px-4 sm:table-cell",
          width: "12%",
        },
      },
      ...(!isPlatformResources
        ? [
            {
              cell: ({ row }: { row: { original: ResourceRecord } }) => (
                <Badge
                  tone={
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
              meta: {
                cellClassName: "hidden px-4 py-2.5 sm:table-cell",
                colClassName: "hidden sm:table-column",
                headerClassName: "hidden px-4 sm:table-cell",
                width: "13%",
              },
            },
          ]
        : []),
      {
        cell: ({ row }) => (
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">
            {formatDateTime(row.original.updatedAt, {
              day: "2-digit",
              hour: "2-digit",
              minute: "2-digit",
              month: "short",
              year: "numeric",
            })}
          </span>
        ),
        header: pageT("columns.updatedAt"),
        id: "updatedAt",
        meta: {
          align: "right",
          cellClassName: "hidden whitespace-nowrap px-4 py-2.5 xl:table-cell",
          colClassName: "hidden xl:table-column",
          headerClassName: "hidden px-4 xl:table-cell",
          width: "14%",
        },
      },
    ],
    [formatDateTime, isPlatformResources, t],
  );

  const resourceTable = useServerTable({
    columns,
    enabled: Boolean(activeWorkspaceCode),
    getRowId: (row) => `${row.workspaceCode}:${row.resourceCode}`,
    params: resourceQueryParams,
    queryFn: ({ pageIndex, pageSize, params }) =>
      queryResourcePage({
        ...params,
        current: pageIndex + 1,
        size: pageSize,
      }),
    resource: "resources",
  });

  function openResource(record: ResourceRecord) {
    navigate(
      `${basePath}/detail?resourceCode=${encodeURIComponent(
        record.resourceCode,
      )}`,
    );
  }

  async function createQuickResource(command: QuickResourceCreateCommand) {
    if (isApi) {
      const result = await createResource({
        allowInheritance: command.allowInheritance,
        description: command.description,
        file: command.file,
        kind: command.kind,
        name: command.name,
        resourceCode: command.resourceCode,
        workspaceCode: command.workspaceCode,
      });
      return {
        kind: fromResourceKindCode(result.kind),
        resourceCode: result.resourceCode,
      };
    }

    const created = createMockQuickResource({
      allowInheritance: command.allowInheritance,
      description: command.description,
      file: command.file,
      kind: command.kind,
      name: command.name,
      pageCount: null,
      resourceCode: command.resourceCode,
      workspaceCode: activeWorkspaceCode,
    });
    return { kind: created.kind, resourceCode: created.resourceCode };
  }

  function continueQuickResourceInEditor(
    payload: QuickResourceUploadHandoffPayload,
  ) {
    const handoffId = useQuickResourceUploadHandoffStore
      .getState()
      .put(payload);
    setQuickUploadOpen(false);
    navigate(`${basePath}/new?quickUpload=${encodeURIComponent(handoffId)}`);
  }

  function openQuickResource(result: { resourceCode: string }) {
    setQuickUploadOpen(false);
    void resourceTable.refresh();
    navigate(
      `${basePath}/detail?resourceCode=${encodeURIComponent(
        result.resourceCode,
      )}`,
    );
    toast.success(
      pageT("quickUpload.messages.created", { name: result.resourceCode }),
    );
  }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] flex-col">
      <div className="space-y-5">
        <TanStackDataTable<ResourceRecord>
          variant="embedded"
          table={resourceTable.table}
          loading={resourceTable.isLoading}
          error={resourceTable.error}
          title={
            isFileManagement
              ? t("pages:resourceConnections.file.title", {
                  defaultValue: "File Management",
                })
              : isPlatformResources
              ? pageT("title.platform", { defaultValue: "Platform Resources" })
              : pageT("title.workspace")
          }
          toolbar={
            <div className="flex gap-2">
              <TextInput
                className="w-full"
                placeholder={pageT("search.placeholder")}
                type="search"
                value={query}
                onChange={(event) => {
                  setQuery(event.target.value);
                }}
              />
              <SelectInput
                aria-label={pageT("filters.kindAria")}
                className="w-full"
                value={kindFilter}
                onValueChange={(value) => {
                  setKindFilter(value ?? "ALL");
                }}
              >
                <SelectItem value="ALL">
                  {pageT("filters.allKinds")}
                </SelectItem>
                {kindOptions.map((kind) => (
                  <SelectItem key={kind} value={kind}>
                    {pageT(`kind.${kind.toLowerCase()}`)}
                  </SelectItem>
                ))}
              </SelectInput>
              {!isPlatformResources ? (
                <SelectInput
                  aria-label={pageT("filters.sourceAria")}
                  className="w-full"
                  value={contextFilter}
                  onValueChange={(value) => {
                    setContextFilter(value as ResourceSourceKind | "ALL");
                  }}
                >
                  {(["ALL", "LOCAL", "INHERITED", "OVERRIDE"] as const).map(
                    (value) => (
                      <SelectItem key={value} value={value}>
                        {value === "ALL"
                          ? pageT("filters.allSources")
                          : pageT(`source.${value.toLowerCase()}`)}
                      </SelectItem>
                    ),
                  )}
                </SelectInput>
              ) : null}
              <Button
                size="lg"
                disabled={!activeWorkspace}
                variant="default"
                onClick={() => setQuickUploadOpen(true)}
              >
                {pageT("actions.quickUpload")}
              </Button>
            </div>
          }
          pagination={
            <TanStackTablePagination
              itemLabel={pageT("pagination.itemLabel")}
              pageSizeOptions={[15, 25, 50]}
              showRange={false}
              table={resourceTable.table}
              totalItems={resourceTable.data.total}
            />
          }
          onRowClick={openResource}
          emptyState={
            activeWorkspace ? pageT("empty") : pageT("emptyWorkspace")
          }
          tableClassName="min-w-full table-fixed max-sm:w-[calc(100%-1px)] max-sm:min-w-0 max-sm:[&>colgroup]:hidden xl:min-w-[900px]"
        />
      </div>
      <QuickResourceUploadDialog
        open={quickUploadOpen}
        workspaceCode={activeWorkspaceCode}
        onClose={() => setQuickUploadOpen(false)}
        onContinueInEditor={
          isFileManagement ? undefined : continueQuickResourceInEditor
        }
        onCreate={createQuickResource}
        onCreated={openQuickResource}
      />
    </div>
  );
}
