import {
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
  resourceLifecycleFilters,
  resourceScopeFilters,
  resourceSourceFilters,
  resourceValidationFilters,
} from "@/domain/resource/constants";
import {
  useResourceDefinitionVersions,
  usePlatformResourceDefinitionVersions,
  usePlatformResourceList,
  useResourceList,
} from "@/domain/resource/hooks";
import type {
  ResourceContextFilter,
  ResourceLifecycleFilter,
  ResourceValidationFilter,
  ResourceVersionRecord,
} from "@/domain/resource/model";
import {
  DataTable,
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextInput,
  useLocaleFormatters,
  useTablePagination,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";

export function ResourcesPage() {
  const { t } = useTranslation(["pages", "common"]);
  const toast = useToast();
  const location = useLocation();
  const navigate = useNavigate();
  const { formatDateTime } = useLocaleFormatters();
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const isPlatform = location.pathname.startsWith("/settings/");
  const basePath = isPlatform ? "/settings/resources" : "/resources";
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const activeWorkspaceName = activeWorkspace?.name ?? "";
  const workspaceRecords = useResourceList(activeWorkspaceCode, activeWorkspaceName);
  const platformRecords = usePlatformResourceList();
  const records = isPlatform ? platformRecords : workspaceRecords;
  const workspaceDefinitionRecords = useResourceDefinitionVersions(activeWorkspaceCode, activeWorkspaceName);
  const platformDefinitionRecords = usePlatformResourceDefinitionVersions();
  const definitionRecords = isPlatform ? platformDefinitionRecords : workspaceDefinitionRecords;
  const [query, setQuery] = useState("");
  const [lifecycleFilter, setLifecycleFilter] = useState<ResourceLifecycleFilter>("ACTIVE");
  const [definitionFilter, setDefinitionFilter] = useState("ALL");
  const [contextFilter, setContextFilter] = useState<ResourceContextFilter>("ALL");
  const [validationFilter, setValidationFilter] = useState<ResourceValidationFilter>("ALL");

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

  const filteredRecords = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return records.filter((record) => {
      if (lifecycleFilter !== "ALL" && record.lifecycleStatus !== lifecycleFilter) return false;
      if (definitionFilter !== "ALL" && record.definitionCode !== definitionFilter) return false;
      if (showEffectiveSource && contextFilter !== "ALL" && record.sourceKind !== contextFilter) return false;
      if (!showEffectiveSource && contextFilter !== "ALL" && record.scopeKind !== contextFilter) return false;
      if (showValidationFilter && validationFilter !== "ALL" && record.validationStatus !== validationFilter) return false;
      if (!normalizedQuery) return true;

      return [record.resourceCode, record.name, record.description]
        .some((value) => value.toLowerCase().includes(normalizedQuery));
    });
  }, [contextFilter, definitionFilter, lifecycleFilter, query, records, showEffectiveSource, showValidationFilter, validationFilter]);
  const pagination = useTablePagination(filteredRecords);

  function changeLifecycle(value: ResourceLifecycleFilter) {
    setLifecycleFilter(value);
    setContextFilter("ALL");
    if (value !== "DRAFT") setValidationFilter("ALL");
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

  function announceAction(record: ResourceVersionRecord, action: string) {
    toast.info(pageT(`messages.${action}`, {
      name: record.name,
      version: record.resourceVersion,
    }));
  }

  function openResource(record: ResourceVersionRecord) {
    navigate(`${basePath}/detail?resourceCode=${encodeURIComponent(record.resourceCode)}&resourceVersion=${encodeURIComponent(record.resourceVersion)}`);
  }

  function renderActions(record: ResourceVersionRecord) {
    const stopPropagation = (event: React.SyntheticEvent) => event.stopPropagation();

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
                <DropdownMenuItem onClick={() => openResource(record)}>{pageT("actions.editDraft")}</DropdownMenuItem>
                <DropdownMenuItem onClick={() => announceAction(record, "publish")}>{pageT("actions.publish")}</DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem variant="destructive" onClick={() => announceAction(record, "discardDraft")}>
                  {pageT("actions.discardDraft")}
                </DropdownMenuItem>
              </>
            ) : (
              <>
                <DropdownMenuItem onClick={() => openResource(record)}>{pageT("actions.open")}</DropdownMenuItem>
                <DropdownMenuItem onClick={() => announceAction(
                  record,
                  record.lifecycleStatus === "ACTIVE" ? "createDraft" : "activate",
                )}
                >
                  {pageT(record.lifecycleStatus === "ACTIVE" ? "actions.createDraft" : "actions.activate")}
                </DropdownMenuItem>
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
            <Button variant="outline" onClick={() => toast.success(pageT("messages.refreshed"))}>
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
          <div className={`grid gap-4 md:grid-cols-2 ${showValidationFilter ? "xl:grid-cols-5" : "xl:grid-cols-4"}`}>
            <div className="min-w-0">
              <Field label={pageT("search.label")}>
                <TextInput
                  className="w-full"
                  placeholder={pageT("search.placeholder")}
                  type="search"
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
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
                  onValueChange={(value) => setDefinitionFilter(value ?? "ALL")}
                >
                  <SelectItem value="ALL">{pageT("filters.allDefinitions")}</SelectItem>
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
                  onValueChange={(value) => setContextFilter(value as ResourceContextFilter)}
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
                    onValueChange={(value) => setValidationFilter(value as ResourceValidationFilter)}
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
          rows={pagination.visibleItems}
          pagination={(
            <TablePagination
              itemLabel={pageT("pagination.itemLabel")}
              showRange={false}
              {...pagination.paginationProps}
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
    </div>
  );
}
