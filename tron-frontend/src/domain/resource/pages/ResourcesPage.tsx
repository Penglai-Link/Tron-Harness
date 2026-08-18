import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  useLocation,
  useNavigate,
} from "react-router";
import {
  MoreHorizontal,
  Upload,
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
  Button,
  buttonVariants,
} from "@/components/ui/button";
import { ResourceVersionSource } from "@/domain/resource/components";
import {
  QuickResourceUploadDialog,
  type QuickResourceCreateCommand,
} from "@/domain/resource/features/quick-upload";
import {
  activateResourceVersion,
  createResource,
  createResourceDraft,
  discardResourceDraft,
  pageResources,
  publishResourceDraft,
} from "@/domain/resource/api";
import {
  resourceLifecycleFilters,
  resourceScopeFilters,
  resourceSourceFilters,
  resourceValidationFilters,
} from "@/domain/resource/constants";
import {
  useActivePlatformResourceDefinitions,
  useActivePlatformResourceDefinitionRecords,
  useActiveResourceDefinitions,
  useActiveResourceDefinitionRecords,
  useResourcePage,
} from "@/domain/resource/hooks";
import { createMockQuickResource } from "@/domain/resource/mock";
import type {
  ResourceContextFilter,
  ResourceLifecycleFilter,
  QuickResourceUploadHandoffPayload,
  ResourceValidationFilter,
  ResourceVersionRecord,
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

type ResourceListAction = "activate" | "createDraft" | "discard" | "publish";

export function ResourcesPage() {
  const { t } = useTranslation(["pages", "common"]);
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const { formatDateTime } = useLocaleFormatters();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const isPlatform = location.pathname.startsWith("/settings/");
  const isFileManagement = location.pathname.startsWith("/resource-management/files");
  const basePath = isPlatform ? "/settings/resources" : isFileManagement ? "/resource-management/files" : "/resources";
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const activeWorkspaceName = activeWorkspace?.name ?? "";
  const scopeKind = isPlatform ? "PLATFORM" as const : "WORKSPACE" as const;
  const isApi = getServiceDataSource("resource") === "api";
  const [query, setQuery] = useState("");
  const [lifecycleFilter, setLifecycleFilter] = useState<ResourceLifecycleFilter>("ACTIVE");
  const [definitionFilter, setDefinitionFilter] = useState(isFileManagement ? "FILE" : "ALL");
  const [contextFilter, setContextFilter] = useState<ResourceContextFilter>("ALL");
  const [validationFilter, setValidationFilter] = useState<ResourceValidationFilter>("ALL");
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [quickUploadOpen, setQuickUploadOpen] = useState(false);
  const resourcePage = useResourcePage({
    context: contextFilter,
    current,
    definitionCode: definitionFilter === "ALL" ? null : definitionFilter,
    lifecycle: lifecycleFilter,
    scopeKind,
    search: query,
    size,
    validation: validationFilter,
    workspaceCode: isPlatform ? null : activeWorkspaceCode || null,
    workspaceName: activeWorkspaceName,
  });
  const workspaceDefinitionRecords = useActiveResourceDefinitionRecords(
    isPlatform ? "" : activeWorkspaceCode,
    isPlatform ? "" : activeWorkspaceName,
  );
  const platformDefinitionRecords = useActivePlatformResourceDefinitionRecords(isPlatform);
  const workspaceQuickUploadDefinitions = useActiveResourceDefinitions(
    isPlatform ? "" : activeWorkspaceCode,
    isPlatform ? "" : activeWorkspaceName,
  );
  const platformQuickUploadDefinitions = useActivePlatformResourceDefinitions();
  const records = resourcePage.data.records;
  const definitionQuery = isPlatform ? platformDefinitionRecords : workspaceDefinitionRecords;
  const definitionRecords = definitionQuery.data.filter((record) => (
    !isFileManagement || ["FILE", "IMAGE", "PDF"].includes(record.definitionCode)
  ));
  const quickUploadDefinitions = (isPlatform ? platformQuickUploadDefinitions : workspaceQuickUploadDefinitions)
    .filter((record) => !isFileManagement || ["FILE", "IMAGE", "PDF"].includes(record.definitionCode));

  const pageT = (key: string, options?: Record<string, unknown>) => (
    t(`pages:resources.${key}`, options)
  );
  const showEffectiveSource = lifecycleFilter === "ACTIVE";
  const showValidationFilter = lifecycleFilter === "DRAFT";
  const contextOptions: ResourceContextFilter[] = isPlatform
    ? ["ALL", "PLATFORM"]
    : showEffectiveSource ? resourceSourceFilters : resourceScopeFilters;

  const definitionOptions = useMemo(() => {
    const definitions = new Map<string, string>();
    definitionRecords
      .filter((record) => record.lifecycleStatus === "ACTIVE")
      .forEach((record) => definitions.set(record.definitionCode, record.name));
    records.forEach((record) => {
      if (!definitions.has(record.definitionCode)) {
        definitions.set(record.definitionCode, record.definitionName);
      }
    });
    return Array.from(definitions, ([value, label]) => ({ label, value }))
      .sort((left, right) => left.label.localeCompare(right.label));
  }, [definitionRecords, records]);

  const pageCount = Math.max(1, Math.ceil(resourcePage.data.total / size));
  const startItem = resourcePage.data.total ? (current - 1) * size + 1 : 0;
  const endItem = Math.min(current * size, resourcePage.data.total);

  useEffect(() => {
    if (!resourcePage.loading && current > pageCount) setCurrent(pageCount);
  }, [current, pageCount, resourcePage.loading]);

  function changeLifecycle(value: ResourceLifecycleFilter) {
    setLifecycleFilter(value);
    setContextFilter("ALL");
    if (value !== "DRAFT") setValidationFilter("ALL");
    setCurrent(1);
  }

  function renderState(record: ResourceVersionRecord) {
    const lifecycleTone = record.lifecycleStatus === "ACTIVE"
      ? "success"
      : record.lifecycleStatus === "DRAFT"
        ? "warning"
        : "info";

    return (
      <div className="flex flex-wrap items-center gap-1.5">
        <Badge tone={lifecycleTone}>{pageT(`lifecycle.${record.lifecycleStatus.toLowerCase()}`)}</Badge>
        <Badge tone={record.validationStatus === "VALID" ? "success" : "error"}>
          {pageT(`validation.${record.validationStatus.toLowerCase()}`)}
        </Badge>
        {record.draftRevision !== null ? (
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">
            {pageT("draftRevision", { revision: record.draftRevision })}
          </span>
        ) : null}
      </div>
    );
  }

  function openResource(record: ResourceVersionRecord) {
    navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(record.resourceCode)}&resourceVersion=${encodeURIComponent(record.resourceVersion)}`);
  }

  function actionKey(record: ResourceVersionRecord, action: ResourceListAction) {
    return `${record.scopeKind}:${record.resourceCode}:${record.resourceVersion}:${action}`;
  }

  function apiScope() {
    return {
      scopeKind,
      workspaceCode: isPlatform ? null : activeWorkspaceCode,
    };
  }

  async function activeResourceVersion(record: ResourceVersionRecord) {
    const activePage = await pageResources({
      ...apiScope(),
      current: 1,
      definitionCode: null,
      definitionStatus: null,
      lifecycleStatus: "ACTIVE",
      resourceCode: record.resourceCode,
      search: null,
      size: 1,
      sourceKind: null,
      validationStatus: null,
    });
    return activePage.records.find((item) => item.scopeKind === scopeKind)?.resourceVersion ?? null;
  }

  async function executeAction(record: ResourceVersionRecord, action: ResourceListAction) {
    if (!isApi) {
      openResource(record);
      return;
    }

    const key = actionKey(record, action);
    setPendingAction(key);
    try {
      if (action === "createDraft") {
        const draft = await createResourceDraft({
          ...apiScope(),
          allowInheritance: null,
          attributes: null,
          definitionCode: null,
          description: record.description,
          name: record.name,
          resourceCode: record.resourceCode,
          sourceResourceVersion: record.resourceVersion,
        });
        resourcePage.refresh();
        navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(draft.resourceCode)}&resourceVersion=${encodeURIComponent(draft.resourceVersion)}`);
        toast.success(pageT("messages.createdDraft", { name: record.name, version: draft.resourceVersion }));
        return;
      }

      if (action === "publish") {
        if (record.draftRevision === null) throw new Error(pageT("messages.missingDraftRevision"));
        await publishResourceDraft({
          ...apiScope(),
          expectedDraftRevision: String(record.draftRevision),
          resourceCode: record.resourceCode,
          resourceVersion: record.resourceVersion,
        });
        resourcePage.refresh();
        toast.success(pageT("messages.published", { name: record.name, version: record.resourceVersion }));
        return;
      }

      if (action === "discard") {
        if (record.draftRevision === null) throw new Error(pageT("messages.missingDraftRevision"));
        await discardResourceDraft({
          ...apiScope(),
          expectedDraftRevision: String(record.draftRevision),
          resourceCode: record.resourceCode,
          resourceVersion: record.resourceVersion,
        });
        resourcePage.refresh();
        toast.success(pageT("messages.discarded", { name: record.name, version: record.resourceVersion }));
        return;
      }

      await activateResourceVersion({
        ...apiScope(),
        expectedActiveResourceVersion: await activeResourceVersion(record),
        resourceCode: record.resourceCode,
        resourceVersion: record.resourceVersion,
      });
      resourcePage.refresh();
      toast.success(pageT("messages.activated", { name: record.name, version: record.resourceVersion }));
    } catch (cause) {
      toast.error(cause instanceof Error ? cause.message : pageT("messages.actionFailed"));
    } finally {
      setPendingAction((currentAction) => currentAction === key ? null : currentAction);
    }
  }

  async function createQuickResource(command: QuickResourceCreateCommand) {
    if (isApi) {
      const result = await createResource({
        allowInheritance: command.allowInheritance,
        attributes: command.attributes,
        binaryFiles: [{ binaryName: command.binaryName, file: command.file }],
        definitionCode: command.definitionCode,
        description: command.description,
        expectedActiveResourceVersion: null,
        name: command.name,
        removedBinaryNames: [],
        resourceCode: command.resourceCode,
        scopeKind: command.scopeKind,
        workspaceCode: command.workspaceCode,
      });
      return { resourceCode: result.resourceCode, resourceVersion: result.resourceVersion };
    }

    const definition = quickUploadDefinitions.find((item) => item.definitionCode === command.definitionCode);
    if (!definition) throw new Error(pageT("quickUpload.blocked.definition_unavailable"));
    const created = createMockQuickResource({
      allowInheritance: command.allowInheritance,
      attributes: command.attributes,
      binaryName: command.binaryName,
      byteSize: command.file.size,
      definition,
      description: command.description,
      name: command.name,
      resourceCode: command.resourceCode,
      scopeCode: isPlatform ? "PLATFORM" : activeWorkspaceCode,
      scopeKind,
    });
    return { resourceCode: created.resourceCode, resourceVersion: created.resourceVersion };
  }

  function continueQuickResourceInEditor(payload: QuickResourceUploadHandoffPayload) {
    const handoffId = useQuickResourceUploadHandoffStore.getState().put(payload);
    setQuickUploadOpen(false);
    navigate(`${basePath}/new?quickUpload=${encodeURIComponent(handoffId)}`);
  }

  function openQuickResource(result: { resourceCode: string; resourceVersion: string }) {
    setQuickUploadOpen(false);
    resourcePage.refresh();
    navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(result.resourceCode)}&resourceVersion=${encodeURIComponent(result.resourceVersion)}`);
    toast.success(pageT("quickUpload.messages.created", { name: result.resourceCode }));
  }

  function renderActions(record: ResourceVersionRecord) {
    const stopPropagation = (event: React.SyntheticEvent) => event.stopPropagation();
    const disabled = pendingAction !== null;

    return (
      <div className="flex justify-end" onClick={stopPropagation} onKeyDown={stopPropagation}>
        <DropdownMenu>
          <DropdownMenuTrigger
            aria-label={pageT("actions.openMenu", {
              name: record.name,
              version: record.resourceVersion,
            })}
            className={buttonVariants({ size: "icon", variant: "ghost" })}
          >
            <MoreHorizontal aria-hidden="true" size={16} />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-44" onClick={stopPropagation}>
            {record.lifecycleStatus === "DRAFT" ? (
              <>
                <DropdownMenuItem disabled={disabled} onClick={() => openResource(record)}>{pageT("actions.editDraft")}</DropdownMenuItem>
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
                <DropdownMenuItem disabled={disabled} onClick={() => openResource(record)}>{pageT("actions.open")}</DropdownMenuItem>
                {isApi ? (
                  <DropdownMenuItem
                    disabled={disabled}
                    onClick={() => { void executeAction(record, record.lifecycleStatus === "ACTIVE" ? "createDraft" : "activate"); }}
                  >
                    {pageT(record.lifecycleStatus === "ACTIVE" ? "actions.createDraft" : "actions.activate")}
                  </DropdownMenuItem>
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
        title={isFileManagement ? t("pages:resourceConnections.file.title", { defaultValue: "File Management" }) : pageT(isPlatform ? "title.platform" : "title.workspace")}
        actions={(
          <>
            <Button variant="outline" onClick={() => {
              resourcePage.refresh();
              definitionQuery.refresh();
              toast.success(pageT("messages.refreshed"));
            }}>
              <RefreshCcw aria-hidden="true" size={15} />
              {t("common:actions.refresh")}
            </Button>
            <Button
              disabled={!isPlatform && !activeWorkspace}
              variant={isFileManagement ? "default" : "outline"}
              onClick={() => setQuickUploadOpen(true)}
            >
              <Upload aria-hidden="true" size={15} />
              {pageT("actions.quickUpload")}
            </Button>
            {!isFileManagement ? (
              <Button disabled={!isPlatform && !activeWorkspace} onClick={() => navigate(`${basePath}/new`)}>
                <Plus aria-hidden="true" size={15} />
                {pageT("actions.new")}
              </Button>
            ) : null}
          </>
        )}
      />

      <div className="space-y-5">
        <SurfaceCard>
          <div className={`grid gap-4 md:grid-cols-2 ${showValidationFilter ? "xl:grid-cols-5" : "xl:grid-cols-4"}`}>
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
                  onValueChange={(value) => changeLifecycle(value as ResourceLifecycleFilter)}
                >
                  {resourceLifecycleFilters.map((value) => (
                    <SelectItem key={value} value={value}>
                      {value === "ALL" ? pageT("filters.displayAll") : pageT(`filters.lifecycle.${value.toLowerCase()}`)}
                    </SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>
            <div className="min-w-0">
              <Field label={pageT("filters.labels.definition")}>
                <SelectInput
                  aria-label={pageT("filters.definitionAria")}
                  className="w-full"
                  value={definitionFilter}
                  onValueChange={(value) => {
                    setDefinitionFilter(value ?? "ALL");
                    setCurrent(1);
                  }}
                >
                  {!isFileManagement ? <SelectItem value="ALL">{pageT("filters.allDefinitions")}</SelectItem> : null}
                  {definitionOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>
            <div className="min-w-0">
              <Field label={pageT(showEffectiveSource ? "filters.labels.source" : "filters.labels.scope")}>
                <SelectInput
                  aria-label={pageT(showEffectiveSource ? "filters.sourceAria" : "filters.scopeAria")}
                  className="w-full"
                  value={contextFilter}
                  onValueChange={(value) => {
                    setContextFilter(value as ResourceContextFilter);
                    setCurrent(1);
                  }}
                >
                  {contextOptions.map((value) => (
                    <SelectItem key={value} value={value}>
                      {value === "ALL"
                        ? pageT(showEffectiveSource ? "filters.allSources" : "filters.allScopes")
                        : pageT(`source.${value.toLowerCase()}`)}
                    </SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>
            {showValidationFilter ? (
              <div className="min-w-0">
                <Field label={pageT("filters.labels.validation")}>
                  <SelectInput
                    aria-label={pageT("filters.validationAria")}
                    className="w-full"
                    value={validationFilter}
                    onValueChange={(value) => {
                      setValidationFilter(value as ResourceValidationFilter);
                      setCurrent(1);
                    }}
                  >
                    {resourceValidationFilters.map((value) => (
                      <SelectItem key={value} value={value}>
                        {value === "ALL" ? pageT("filters.allValidation") : pageT(`validation.${value.toLowerCase()}`)}
                      </SelectItem>
                    ))}
                  </SelectInput>
                </Field>
              </div>
            ) : null}
          </div>
        </SurfaceCard>

        <DataTable<ResourceVersionRecord>
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
          rowKey={(row) => `${row.scopeKind}:${row.ownerWorkspaceCode ?? "PLATFORM"}:${row.resourceCode}:${row.resourceVersion}`}
          onRowClick={openResource}
          columns={[
            {
              id: "resource",
              header: pageT("columns.resource"),
              width: "30%",
              headerClassName: "px-4 max-sm:w-[58%] max-sm:px-3",
              cellClassName: "px-4 py-2.5 max-sm:w-[58%] max-sm:px-3",
              render: (row) => (
                <div className="min-w-0">
                  <p className="truncate font-medium text-[var(--color-on-surface)]" title={row.name}>{row.name}</p>
                  <div className="mt-1 flex min-w-0 items-center gap-2 text-[12px] text-[var(--color-on-surface-variant)]">
                    <span className="shrink-0 font-mono sm:hidden">{row.resourceVersion}</span>
                    <span aria-hidden="true" className="text-[var(--color-outline)] sm:hidden">·</span>
                    <span className="truncate font-mono" title={row.resourceCode}>{row.resourceCode}</span>
                    <span aria-hidden="true" className="hidden text-[var(--color-outline)] xl:inline">·</span>
                    <span className="hidden truncate xl:inline" title={row.description}>{row.description}</span>
                  </div>
                  <p className="mt-1 truncate text-[12px] text-[var(--color-on-surface-variant)] lg:hidden" title={row.definitionName}>
                    {row.definitionName}
                  </p>
                  <div className="mt-1 sm:hidden"><ResourceVersionSource inline record={row} /></div>
                </div>
              ),
            },
            {
              id: "definition",
              header: pageT("columns.definition"),
              width: "17%",
              colClassName: "hidden lg:table-column",
              headerClassName: "hidden px-4 lg:table-cell",
              cellClassName: "hidden px-4 py-2.5 lg:table-cell",
              render: (row) => (
                <div className="min-w-0">
                  <p className="truncate text-[13px] font-medium" title={row.definitionName}>{row.definitionName}</p>
                  <p className="mt-1 truncate font-mono text-[11px] text-[var(--color-on-surface-variant)]" title={row.definitionCode}>{row.definitionCode}</p>
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
              render: (row) => <span className="font-mono text-[13px]">{row.resourceVersion}</span>,
            },
            {
              id: "state",
              header: pageT("columns.state"),
              width: "13%",
              headerClassName: "px-4 max-sm:w-[25%] max-sm:px-2",
              cellClassName: "px-4 py-2.5 max-sm:w-[25%] max-sm:px-2",
              render: renderState,
            },
            {
              id: "source",
              header: pageT(showEffectiveSource ? "columns.source" : "columns.scope"),
              width: "13%",
              colClassName: "hidden sm:table-column",
              headerClassName: "hidden px-4 sm:table-cell",
              cellClassName: "hidden px-4 py-2.5 sm:table-cell",
              render: (row) => <ResourceVersionSource record={row} />,
            },
            {
              id: "updatedAt",
              header: pageT("columns.updatedAt"),
              align: "right",
              width: "12%",
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
      <QuickResourceUploadDialog
        definitions={quickUploadDefinitions}
        isPlatform={isPlatform}
        open={quickUploadOpen}
        scopeCode={isPlatform ? "PLATFORM" : activeWorkspaceCode}
        scopeKind={scopeKind}
        onClose={() => setQuickUploadOpen(false)}
        onContinueInEditor={isFileManagement ? undefined : continueQuickResourceInEditor}
        onCreate={createQuickResource}
        onCreated={openQuickResource}
      />
    </div>
  );
}
