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
  MoreHorizontal,
  Plus,
  RefreshCcw,
} from "lucide-react";
import { SelectItem } from "@/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

import {
  ConfirmDialog,
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
import {
  Button,
  buttonVariants,
} from "@/components/ui/button";
import { ResourceDefinitionSource } from "@/domain/resource/components";
import {
  activateResourceDefinitionVersion,
  createResourceDefinitionDraft,
  deleteResourceDefinition,
  discardResourceDefinitionDraft,
  pageResourceDefinitions,
  publishResourceDefinitionDraft,
  setResourceDefinitionEnabled,
} from "@/domain/resource/api";
import {
  useResourceDefinitionPage,
} from "@/domain/resource/hooks";
import type {
  ResourceDefinitionEnabledFilter,
  ResourceDefinitionLifecycleFilter,
  ResourceDefinitionValidationFilter,
  ResourceDefinitionVersionRecord,
} from "@/domain/resource/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Badge } from "@/shared/ui/Badge";

const lifecycleFilters: ResourceDefinitionLifecycleFilter[] = ["ACTIVE", "DRAFT", "HISTORICAL", "ALL"];
const enabledFilters: ResourceDefinitionEnabledFilter[] = ["ALL", "ENABLED", "DISABLED"];
const validationFilters: ResourceDefinitionValidationFilter[] = ["ALL", "VALID", "INVALID"];
type ResourceDefinitionListAction = "activate" | "createDraft" | "delete" | "discard" | "publish" | "setEnabled";

export function ResourceDefinitionsPage() {
  const { t } = useTranslation(["pages", "common"]);
  const location = useLocation();
  const navigate = useNavigate();
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const isPlatform = location.pathname.startsWith("/settings/");
  const basePath = isPlatform ? "/settings/resource-definitions" : "/resource-definitions";
  const workspaceCode = isPlatform ? "PLATFORM" : activeWorkspace?.workspaceCode ?? "";
  const workspaceName = isPlatform ? "Platform" : activeWorkspace?.name ?? "";
  const scopeKind = isPlatform ? "PLATFORM" as const : "WORKSPACE" as const;
  const isApi = getServiceDataSource("resource") === "api";
  const [query, setQuery] = useState("");
  const [lifecycleFilter, setLifecycleFilter] = useState<ResourceDefinitionLifecycleFilter>("ACTIVE");
  const [enabledFilter, setEnabledFilter] = useState<ResourceDefinitionEnabledFilter>("ALL");
  const [validationFilter, setValidationFilter] = useState<ResourceDefinitionValidationFilter>("ALL");
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<ResourceDefinitionVersionRecord | null>(null);
  const page = useResourceDefinitionPage({
    current,
    enabled: enabledFilter,
    lifecycle: lifecycleFilter,
    scopeKind,
    search: query,
    size,
    validation: validationFilter,
    workspaceCode: isPlatform ? null : workspaceCode || null,
    workspaceName,
  });
  const records = page.data.records;

  const pageT = (key: string, options?: Record<string, unknown>) => (
    t(`pages:resourceDefinitions.${key}`, options)
  );

  const pageCount = Math.max(1, Math.ceil(page.data.total / size));
  const startItem = page.data.total ? (current - 1) * size + 1 : 0;
  const endItem = Math.min(current * size, page.data.total);

  useEffect(() => {
    if (!page.loading && current > pageCount) setCurrent(pageCount);
  }, [current, page.loading, pageCount]);

  function renderLifecycle(record: ResourceDefinitionVersionRecord) {
    const tone = record.lifecycleStatus === "ACTIVE"
      ? "success"
      : record.lifecycleStatus === "DRAFT"
        ? "warning"
        : "info";

    return (
      <div className="flex flex-wrap items-center gap-1.5">
        <Badge tone={tone}>{pageT(`lifecycle.${record.lifecycleStatus.toLowerCase()}`)}</Badge>
        {record.lifecycleStatus === "DRAFT" ? (
          <Badge tone={record.validationStatus === "VALID" ? "success" : "error"}>
            {pageT(`validation.${record.validationStatus.toLowerCase()}`)}
          </Badge>
        ) : null}
        {record.draftRevision !== null ? (
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">
            {pageT("draftRevision", { revision: record.draftRevision })}
          </span>
        ) : null}
        <span className="inline-flex items-center gap-1 text-[12px] xl:hidden">
          <span
            aria-hidden="true"
            className={record.enabled ? "size-1.5 rounded-full bg-emerald-500" : "size-1.5 rounded-full bg-slate-400"}
          />
          {pageT(`enabled.${record.enabled ? "yes" : "no"}`)}
        </span>
      </div>
    );
  }

  function openDefinition(record: ResourceDefinitionVersionRecord) {
    navigate(`${basePath}/detail?definitionCode=${encodeURIComponent(record.definitionCode)}&definitionVersion=${encodeURIComponent(record.definitionVersion)}`);
  }

  function actionKey(record: ResourceDefinitionVersionRecord, action: ResourceDefinitionListAction) {
    return `${record.scopeKind}:${record.definitionCode}:${record.definitionVersion}:${action}`;
  }

  function apiScope() {
    return {
      scopeKind,
      workspaceCode: isPlatform ? null : workspaceCode,
    };
  }

  async function activeDefinitionVersion(record: ResourceDefinitionVersionRecord) {
    const activePage = await pageResourceDefinitions({
      ...apiScope(),
      current: 1,
      definitionCode: record.definitionCode,
      definitionStatus: null,
      lifecycleStatus: "ACTIVE",
      search: null,
      size: 1,
      sourceKind: null,
      validationStatus: null,
    });
    return activePage.records.find((item) => item.scopeKind === scopeKind)?.definitionVersion ?? null;
  }

  function ownsActiveDefinition(record: ResourceDefinitionVersionRecord) {
    return record.lifecycleStatus === "ACTIVE" && record.scopeKind === scopeKind;
  }

  async function executeAction(record: ResourceDefinitionVersionRecord, action: ResourceDefinitionListAction) {
    if (!isApi) {
      openDefinition(record);
      return;
    }

    const key = actionKey(record, action);
    setPendingAction(key);
    try {
      if (action === "createDraft") {
        const draft = await createResourceDefinitionDraft({
          ...apiScope(),
          allowInheritance: null,
          attributesSchema: null,
          binarySchema: null,
          binaryStorageProfile: null,
          definitionCode: record.definitionCode,
          definitionStatus: null,
          description: record.description,
          name: record.name,
          sourceDefinitionVersion: record.definitionVersion,
        });
        page.refresh();
        navigate(`${basePath}/detail?definitionCode=${encodeURIComponent(draft.definitionCode)}&definitionVersion=${encodeURIComponent(draft.definitionVersion)}`);
        toast.success(pageT("messages.createdDraft", { name: record.name, version: draft.definitionVersion }));
        return;
      }

      if (action === "publish") {
        if (record.draftRevision === null) throw new Error(pageT("messages.missingDraftRevision"));
        await publishResourceDefinitionDraft({
          ...apiScope(),
          definitionCode: record.definitionCode,
          definitionVersion: record.definitionVersion,
          expectedDraftRevision: String(record.draftRevision),
        });
        page.refresh();
        toast.success(pageT("messages.published", { name: record.name, version: record.definitionVersion }));
        return;
      }

      if (action === "discard") {
        if (record.draftRevision === null) throw new Error(pageT("messages.missingDraftRevision"));
        await discardResourceDefinitionDraft({
          ...apiScope(),
          definitionCode: record.definitionCode,
          definitionVersion: record.definitionVersion,
          expectedDraftRevision: String(record.draftRevision),
        });
        page.refresh();
        toast.success(pageT("messages.discarded", { name: record.name, version: record.definitionVersion }));
        return;
      }

      if (action === "setEnabled") {
        if (!ownsActiveDefinition(record)) {
          throw new Error(pageT("messages.setEnabledBlocked"));
        }
        await setResourceDefinitionEnabled({
          ...apiScope(),
          definitionCode: record.definitionCode,
          enabled: !record.enabled,
          expectedActiveDefinitionVersion: record.definitionVersion,
        });
        page.refresh();
        toast.success(pageT(
          record.enabled ? "messages.disabled" : "messages.enabled",
          { name: record.name },
        ));
        return;
      }

      if (action === "delete") {
        if (!ownsActiveDefinition(record) || record.preset || record.enabled) {
          throw new Error(pageT("messages.deleteBlocked"));
        }
        const result = await deleteResourceDefinition({
          ...apiScope(),
          definitionCode: record.definitionCode,
          expectedActiveDefinitionVersion: record.definitionVersion,
        });
        if (!result.deleted) {
          throw new Error(pageT("messages.deleteFailed"));
        }
        setDeleteTarget(null);
        page.refresh();
        toast.success(pageT("messages.deleted", { name: record.name }));
        return;
      }

      await activateResourceDefinitionVersion({
        ...apiScope(),
        definitionCode: record.definitionCode,
        definitionVersion: record.definitionVersion,
        expectedActiveDefinitionVersion: await activeDefinitionVersion(record),
      });
      page.refresh();
      toast.success(pageT("messages.activated", { name: record.name, version: record.definitionVersion }));
    } catch (cause) {
      toast.error(cause instanceof Error ? cause.message : pageT("messages.actionFailed"));
    } finally {
      setPendingAction((currentAction) => currentAction === key ? null : currentAction);
    }
  }

  function renderActions(record: ResourceDefinitionVersionRecord) {
    const stopPropagation = (event: React.SyntheticEvent) => event.stopPropagation();
    const disabled = pendingAction !== null;
    const canManageActive = isApi && ownsActiveDefinition(record);
    const canDelete = canManageActive && !record.preset && !record.enabled;

    return (
      <div className="flex justify-end" onClick={stopPropagation} onKeyDown={stopPropagation}>
        <DropdownMenu>
          <DropdownMenuTrigger
            aria-label={pageT("actions.openMenu", {
              name: record.name,
              version: record.definitionVersion,
            })}
            className={buttonVariants({ size: "icon", variant: "ghost" })}
          >
            <MoreHorizontal aria-hidden="true" size={16} />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-44" onClick={stopPropagation}>
            {record.lifecycleStatus === "DRAFT" ? (
              <>
                <DropdownMenuItem disabled={disabled} onClick={() => openDefinition(record)}>{pageT("actions.editDraft")}</DropdownMenuItem>
                {isApi ? (
                  <>
                    <DropdownMenuItem disabled={disabled} onClick={() => { void executeAction(record, "publish"); }}>{pageT("actions.publish")}</DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem disabled={disabled} variant="destructive" onClick={() => { void executeAction(record, "discard"); }}>
                      {pageT("actions.discardDraft")}
                    </DropdownMenuItem>
                  </>
                ) : null}
              </>
            ) : (
              <>
                <DropdownMenuItem disabled={disabled} onClick={() => openDefinition(record)}>{pageT("actions.open")}</DropdownMenuItem>
                {isApi ? (
                  <>
                    <DropdownMenuItem
                      disabled={disabled}
                      onClick={() => { void executeAction(record, record.lifecycleStatus === "ACTIVE" ? "createDraft" : "activate"); }}
                    >
                      {pageT(record.lifecycleStatus === "ACTIVE" ? "actions.createDraft" : "actions.activate")}
                    </DropdownMenuItem>
                    {canManageActive ? (
                      <>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem disabled={disabled} onClick={() => { void executeAction(record, "setEnabled"); }}>
                          {pageT(record.enabled ? "actions.disable" : "actions.enable")}
                        </DropdownMenuItem>
                        {canDelete ? (
                          <DropdownMenuItem disabled={disabled} variant="destructive" onClick={() => setDeleteTarget(record)}>
                            {pageT("actions.delete")}
                          </DropdownMenuItem>
                        ) : null}
                      </>
                    ) : null}
                  </>
                ) : null}
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    );
  }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] flex-col">
      <PageHeader
        title={pageT(isPlatform ? "title.platform" : "title.workspace")}
        actions={(
          <>
            <Button
              variant="outline"
              onClick={() => {
                page.refresh();
                toast.success(pageT("messages.refreshed"));
              }}
            >
              <RefreshCcw aria-hidden="true" size={15} />
              {t("common:actions.refresh")}
            </Button>
            <Button disabled={!isPlatform && !activeWorkspace} onClick={() => navigate(`${basePath}/new`)}>
              <Plus aria-hidden="true" size={15} />
              {pageT("actions.new")}
            </Button>
          </>
        )}
      />

      <div className="space-y-5">
        <SurfaceCard>
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
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
              <Field label={pageT("filters.labels.lifecycle")}>
                <SelectInput
                  aria-label={pageT("filters.lifecycleAria")}
                  className="w-full"
                  value={lifecycleFilter}
                  onValueChange={(value) => {
                    setLifecycleFilter(value as ResourceDefinitionLifecycleFilter);
                    setCurrent(1);
                  }}
                >
                  {lifecycleFilters.map((value) => (
                    <SelectItem key={value} value={value}>
                      {value === "ALL"
                        ? pageT("filters.displayAll")
                        : pageT(`filters.lifecycle.${value.toLowerCase()}`)}
                    </SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>
            <div className="min-w-0">
              <Field label={pageT("filters.labels.enabled")}>
                <SelectInput
                  aria-label={pageT("filters.enabledAria")}
                  className="w-full"
                  value={enabledFilter}
                  onValueChange={(value) => {
                    setEnabledFilter(value as ResourceDefinitionEnabledFilter);
                    setCurrent(1);
                  }}
                >
                  {enabledFilters.map((value) => (
                    <SelectItem key={value} value={value}>
                      {value === "ALL"
                        ? pageT("filters.displayAll")
                        : pageT(`filters.enabled.${value.toLowerCase()}`)}
                    </SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>
            <div className="min-w-0">
              <Field label={pageT("filters.labels.validation")}>
                <SelectInput
                  aria-label={pageT("filters.validationAria")}
                  className="w-full"
                  value={validationFilter}
                  onValueChange={(value) => {
                    setValidationFilter(value as ResourceDefinitionValidationFilter);
                    setCurrent(1);
                  }}
                >
                  {validationFilters.map((value) => (
                    <SelectItem key={value} value={value}>
                      {value === "ALL"
                        ? pageT("filters.displayAll")
                        : pageT(`filters.validation.${value.toLowerCase()}`)}
                    </SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>
          </div>
        </SurfaceCard>

        <DataTable<ResourceDefinitionVersionRecord>
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
            totalItems={page.data.total}
          />
        )}
        rowKey={(row) => `${row.definitionCode}:${row.definitionVersion}`}
        onRowClick={(row) => navigate(`${basePath}/detail?definitionCode=${encodeURIComponent(row.definitionCode)}&definitionVersion=${encodeURIComponent(row.definitionVersion)}`)}
        columns={[
          {
            id: "definition",
            header: pageT("columns.definition"),
            width: "34%",
            headerClassName: "px-4 max-sm:w-[58%] max-sm:px-3",
            cellClassName: "px-4 py-2.5 max-sm:w-[58%] max-sm:px-3",
            render: (row) => (
              <div className="min-w-0">
                <div className="flex min-w-0 items-center gap-2">
                  <span className="truncate font-medium text-[var(--color-on-surface)]">{row.name}</span>
                  {row.preset ? <span className="hidden sm:inline-flex"><Badge tone="outline">{pageT("preset")}</Badge></span> : null}
                </div>
                <div className="mt-1 flex min-w-0 items-center gap-2 text-[12px] text-[var(--color-on-surface-variant)]">
                  <span className="shrink-0 font-mono sm:hidden">{row.definitionVersion}</span>
                  <span aria-hidden="true" className="text-[var(--color-outline)] sm:hidden">·</span>
                  <span className="shrink-0 font-mono">{row.definitionCode}</span>
                  <span aria-hidden="true" className="hidden text-[var(--color-outline)] md:inline">·</span>
                  <span className="hidden truncate md:inline" title={row.description}>{row.description}</span>
                </div>
                <div className="mt-1 flex min-w-0 flex-wrap items-center gap-1.5 sm:hidden">
                  {row.preset ? <Badge tone="outline">{pageT("preset")}</Badge> : null}
                  <ResourceDefinitionSource inline record={row} />
                </div>
              </div>
            ),
          },
          {
            id: "version",
            header: pageT("columns.version"),
            width: "8%",
            colClassName: "hidden sm:table-column",
            headerClassName: "hidden px-4 sm:table-cell",
            cellClassName: "hidden whitespace-nowrap px-4 py-2.5 sm:table-cell",
            render: (row) => <span className="font-mono text-[13px]">{row.definitionVersion}</span>,
          },
          {
            id: "state",
            header: pageT("columns.state"),
            width: "11%",
            headerClassName: "px-4 max-sm:w-[25%] max-sm:px-2",
            cellClassName: "px-4 py-2.5 max-sm:w-[25%] max-sm:px-2",
            render: renderLifecycle,
          },
          {
            id: "enabled",
            header: pageT("columns.enabled"),
            width: "10%",
            colClassName: "hidden xl:table-column",
            headerClassName: "hidden px-4 xl:table-cell",
            cellClassName: "hidden whitespace-nowrap px-4 py-2.5 xl:table-cell",
            render: (row) => (
              <span className="inline-flex items-center gap-2 text-[13px]">
                <span
                  aria-hidden="true"
                  className={row.enabled ? "size-2 rounded-full bg-emerald-500" : "size-2 rounded-full bg-slate-400"}
                />
                {pageT(`enabled.${row.enabled ? "yes" : "no"}`)}
              </span>
            ),
          },
          {
            id: "source",
            header: pageT("columns.source"),
            width: "13%",
            colClassName: "hidden sm:table-column",
            headerClassName: "hidden px-4 sm:table-cell",
            cellClassName: "hidden px-4 py-2.5 sm:table-cell",
            render: (row) => <ResourceDefinitionSource record={row} />,
          },
          {
            id: "updatedAt",
            header: pageT("columns.updatedAt"),
            align: "right",
            width: "17%",
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
          {
            id: "actions",
            header: pageT("columns.actions"),
            align: "right",
            width: "7%",
            headerClassName: "whitespace-nowrap px-3 max-sm:w-[17%] max-sm:px-1",
            cellClassName: "px-3 py-2.5 max-sm:w-[17%] max-sm:px-1",
            render: renderActions,
          },
        ]}
        emptyState={isPlatform || activeWorkspace ? pageT("empty") : pageT("emptyWorkspace")}
        tableClassName="min-w-full table-fixed max-sm:w-[calc(100%-1px)] max-sm:min-w-0 max-sm:[&>colgroup]:hidden xl:min-w-[900px]"
        />
      </div>
      <ConfirmDialog
        ariaLabel={pageT("deleteDialog.aria")}
        cancelText={pageT("deleteDialog.cancel")}
        confirmText={pendingAction === (deleteTarget ? actionKey(deleteTarget, "delete") : null)
          ? pageT("deleteDialog.deleting")
          : pageT("actions.delete")}
        description={deleteTarget ? pageT("deleteDialog.description", {
          name: deleteTarget.name,
        }) : undefined}
        open={deleteTarget !== null}
        pending={pendingAction === (deleteTarget ? actionKey(deleteTarget, "delete") : null)}
        title={pageT("deleteDialog.title")}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={() => {
          if (deleteTarget) void executeAction(deleteTarget, "delete");
        }}
      />
    </div>
  );
}
