import {
  useEffect,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  useLocation,
  useNavigate,
} from "react-router";
import {
  RefreshCcw,
  Upload,
} from "lucide-react";
import { SelectItem } from "@/components/ui/select";

import { Button } from "@/components/ui/button";
import {
  QuickResourceUploadDialog,
  type QuickResourceCreateCommand,
} from "@/domain/resource/features/quick-upload";
import { createResource, fromResourceKindCode } from "@/domain/resource/api";
import { resourceFileKinds } from "@/domain/resource/constants";
import { useResourcePage, useSettingsResourceScope } from "@/domain/resource/hooks";
import { createMockQuickResource } from "@/domain/resource/mock";
import type {
  QuickResourceUploadHandoffPayload,
  ResourceKind,
  ResourceRecord,
  ResourceSourceKind,
} from "@/domain/resource/model";
import { useQuickResourceUploadHandoffStore } from "@/domain/resource/model";
import {
  DataTable,
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TablePagination,
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
    ? (settingsScope.workspaceCode ? { workspaceCode: settingsScope.workspaceCode, name: settingsScope.workspaceName } : undefined)
    : consoleWorkspace;
  const isFileManagement = location.pathname.startsWith("/resource-management/files");
  const isPlatformResources = settingsScope.isSettings;
  const basePath = settingsScope.isSettings ? "/settings/resources" : isFileManagement ? "/resource-management/files" : "/resources";
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const activeWorkspaceName = activeWorkspace?.name ?? "";
  const isApi = getServiceDataSource("resource") === "api";
  const [query, setQuery] = useState("");
  const [kindFilter, setKindFilter] = useState<string>(isFileManagement ? "FILE" : "ALL");
  const [contextFilter, setContextFilter] = useState<ResourceSourceKind | "ALL">("ALL");
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const [quickUploadOpen, setQuickUploadOpen] = useState(false);
  const kindOptions = isFileManagement ? resourceFileKinds : allResourceKinds;
  const resourcePage = useResourcePage({
    context: isPlatformResources ? "ALL" : contextFilter,
    current,
    kinds: kindFilter === "ALL" ? (isFileManagement ? resourceFileKinds : []) : [kindFilter as ResourceKind],
    search: query,
    size,
    workspaceCode: activeWorkspaceCode || null,
    workspaceName: activeWorkspaceName,
  });
  const records = resourcePage.data.records;

  const pageT = (key: string, options?: Record<string, unknown>) => (
    t(`pages:resources.${key}`, options)
  );

  const pageCount = Math.max(1, Math.ceil(resourcePage.data.total / size));
  const startItem = resourcePage.data.total ? (current - 1) * size + 1 : 0;
  const endItem = Math.min(current * size, resourcePage.data.total);

  useEffect(() => {
    if (!resourcePage.loading && current > pageCount) setCurrent(pageCount);
  }, [current, pageCount, resourcePage.loading]);

  function openResource(record: ResourceRecord) {
    navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(record.resourceCode)}`);
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
      return { kind: fromResourceKindCode(result.kind), resourceCode: result.resourceCode };
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

  function continueQuickResourceInEditor(payload: QuickResourceUploadHandoffPayload) {
    const handoffId = useQuickResourceUploadHandoffStore.getState().put(payload);
    setQuickUploadOpen(false);
    navigate(`${basePath}/new?quickUpload=${encodeURIComponent(handoffId)}`);
  }

  function openQuickResource(result: { resourceCode: string }) {
    setQuickUploadOpen(false);
    resourcePage.refresh();
    navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(result.resourceCode)}`);
    toast.success(pageT("quickUpload.messages.created", { name: result.resourceCode }));
  }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] flex-col">
      <PageHeader
        title={isFileManagement ? t("pages:resourceConnections.file.title", { defaultValue: "File Management" }) : isPlatformResources ? pageT("title.platform", { defaultValue: "Platform Resources" }) : pageT("title.workspace")}
        actions={(
          <>
            <Button variant="outline" onClick={() => {
              resourcePage.refresh();
            }}>
              <RefreshCcw aria-hidden="true" size={15} />
              {t("common:actions.refresh")}
            </Button>
            <Button
              disabled={!activeWorkspace}
              variant="default"
              onClick={() => setQuickUploadOpen(true)}
            >
              <Upload aria-hidden="true" size={15} />
              {pageT("actions.quickUpload")}
            </Button>
          </>
        )}
      />

      <div className="space-y-5">
        <SurfaceCard>
          <div className="grid gap-4 md:grid-cols-3">
            <div className="min-w-0">
              <Field label={pageT("search.label")}>
                <TextInput
                  className="w-full"
                  placeholder={pageT("search.placeholder")}
                  type="search"
                  value={query}
                  onChange={(event) => {
                    setQuery(event.target.value);
                    setCurrent(1);
                  }}
                />
              </Field>
            </div>
            <div className="min-w-0">
              <Field label={pageT("filters.labels.kind")}>
                <SelectInput
                  aria-label={pageT("filters.kindAria")}
                  className="w-full"
                  value={kindFilter}
                  onValueChange={(value) => {
                    setKindFilter(value ?? "ALL");
                    setCurrent(1);
                  }}
                >
                  {!isFileManagement ? <SelectItem value="ALL">{pageT("filters.allKinds")}</SelectItem> : null}
                  {kindOptions.map((kind) => (
                    <SelectItem key={kind} value={kind}>{pageT(`kind.${kind.toLowerCase()}`)}</SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>
            {!isPlatformResources ? (
              <div className="min-w-0">
                <Field label={pageT("filters.labels.source")}>
                  <SelectInput
                    aria-label={pageT("filters.sourceAria")}
                    className="w-full"
                    value={contextFilter}
                    onValueChange={(value) => {
                      setContextFilter(value as ResourceSourceKind | "ALL");
                      setCurrent(1);
                    }}
                  >
                    {(["ALL", "LOCAL", "INHERITED", "OVERRIDE"] as const).map((value) => (
                      <SelectItem key={value} value={value}>
                        {value === "ALL" ? pageT("filters.allSources") : pageT(`source.${value.toLowerCase()}`)}
                      </SelectItem>
                    ))}
                  </SelectInput>
                </Field>
              </div>
            ) : null}
          </div>
        </SurfaceCard>

        <DataTable<ResourceRecord>
          rows={records}
          pagination={(
            <TablePagination
              itemLabel={pageT("pagination.itemLabel")}
              pageSizeOptions={[15, 25, 50]}
              showRange={false}
              endItem={endItem}
              onPageChange={(next) => setCurrent(Math.min(Math.max(next, 1), pageCount))}
              onPageSizeChange={(next) => {
                setSize(next);
                setCurrent(1);
              }}
              page={Math.min(current, pageCount)}
              pageCount={pageCount}
              pageSize={size}
              startItem={startItem}
              totalItems={resourcePage.data.total}
            />
          )}
          rowKey={(row) => `${row.workspaceCode}:${row.resourceCode}`}
          onRowClick={openResource}
          columns={[
            {
              id: "resource",
              header: pageT("columns.resource"),
              width: "32%",
              headerClassName: "px-4 max-sm:w-[58%] max-sm:px-3",
              cellClassName: "px-4 py-2.5 max-sm:w-[58%] max-sm:px-3",
              render: (row) => (
                <div className="min-w-0">
                  <p className="truncate font-medium text-[var(--color-on-surface)]" title={row.name}>{row.name}</p>
                  <div className="mt-1 flex min-w-0 items-center gap-2 text-[12px] text-[var(--color-on-surface-variant)]">
                    <span className="truncate font-mono" title={row.resourceCode}>{row.resourceCode}</span>
                    <span aria-hidden="true" className="hidden text-[var(--color-outline)] xl:inline">·</span>
                    <span className="hidden truncate xl:inline" title={row.description}>{row.description}</span>
                  </div>
                </div>
              ),
            },
            {
              id: "kind",
              header: pageT("columns.kind"),
              width: "17%",
              colClassName: "hidden lg:table-column",
              headerClassName: "hidden px-4 lg:table-cell",
              cellClassName: "hidden px-4 py-2.5 lg:table-cell",
              render: (row) => <Badge tone="outline">{pageT(`kind.${row.kind.toLowerCase()}`)}</Badge>,
            },
            {
              id: "fileSize",
              header: pageT("columns.fileSize"),
              width: "12%",
              colClassName: "hidden sm:table-column",
              headerClassName: "hidden px-4 sm:table-cell",
              cellClassName: "hidden whitespace-nowrap px-4 py-2.5 sm:table-cell",
              render: (row) => (
                <span className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                  {row.file ? formatByteSize(row.file.byteSize) : "—"}
                </span>
              ),
            },
            ...(!isPlatformResources ? [{
              id: "source",
              header: pageT("columns.source"),
              width: "13%",
              colClassName: "hidden sm:table-column",
              headerClassName: "hidden px-4 sm:table-cell",
              cellClassName: "hidden px-4 py-2.5 sm:table-cell",
              render: (row: ResourceRecord) => <Badge tone={row.sourceKind === "LOCAL" ? "success" : row.sourceKind === "INHERITED" ? "info" : row.sourceKind === "OVERRIDE" ? "warning" : "outline"}>{pageT(`source.${row.sourceKind.toLowerCase()}`)}</Badge>,
            }] : []),
            {
              id: "updatedAt",
              header: pageT("columns.updatedAt"),
              align: "right",
              width: "14%",
              colClassName: "hidden xl:table-column",
              headerClassName: "hidden px-4 xl:table-cell",
              cellClassName: "hidden whitespace-nowrap px-4 py-2.5 xl:table-cell",
              render: (row) => (
                <span className="text-[12px] text-[var(--color-on-surface-variant)]">
                  {formatDateTime(row.updatedAt, {
                    day: "2-digit",
                    hour: "2-digit",
                    minute: "2-digit",
                    month: "short",
                    year: "numeric",
                  })}
                </span>
              ),
            },
          ]}
          emptyState={activeWorkspace ? pageT("empty") : pageT("emptyWorkspace")}
          tableClassName="min-w-full table-fixed max-sm:w-[calc(100%-1px)] max-sm:min-w-0 max-sm:[&>colgroup]:hidden xl:min-w-[900px]"
        />
      </div>
      <QuickResourceUploadDialog
        open={quickUploadOpen}
        workspaceCode={activeWorkspaceCode}
        onClose={() => setQuickUploadOpen(false)}
        onContinueInEditor={isFileManagement ? undefined : continueQuickResourceInEditor}
        onCreate={createQuickResource}
        onCreated={openQuickResource}
      />
    </div>
  );
}
