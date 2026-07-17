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
import {
  Button,
  buttonVariants,
} from "@/components/ui/button";
import { ResourceDefinitionSource } from "@/domain/resource/components";
import {
  usePlatformResourceDefinitionVersions,
  useResourceDefinitionVersions,
} from "@/domain/resource/hooks";
import type {
  ResourceDefinitionEnabledFilter,
  ResourceDefinitionLifecycleFilter,
  ResourceDefinitionValidationFilter,
  ResourceDefinitionVersionRecord,
} from "@/domain/resource/model";
import { Badge } from "@/shared/ui/Badge";

const lifecycleFilters: ResourceDefinitionLifecycleFilter[] = ["ACTIVE", "DRAFT", "HISTORICAL", "ALL"];
const enabledFilters: ResourceDefinitionEnabledFilter[] = ["ALL", "ENABLED", "DISABLED"];
const validationFilters: ResourceDefinitionValidationFilter[] = ["ALL", "VALID", "INVALID"];

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
  const sourceRecords = useResourceDefinitionVersions(
    workspaceCode,
    workspaceName,
  );
  const platformRecords = usePlatformResourceDefinitionVersions();
  const records = useMemo(() => (
    isPlatform ? platformRecords : sourceRecords
  ), [isPlatform, platformRecords, sourceRecords]);
  const [query, setQuery] = useState("");
  const [lifecycleFilter, setLifecycleFilter] = useState<ResourceDefinitionLifecycleFilter>("ACTIVE");
  const [enabledFilter, setEnabledFilter] = useState<ResourceDefinitionEnabledFilter>("ALL");
  const [validationFilter, setValidationFilter] = useState<ResourceDefinitionValidationFilter>("ALL");

  const pageT = (key: string, options?: Record<string, unknown>) => (
    t(`pages:resourceDefinitions.${key}`, options)
  );

  const filteredRecords = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return records.filter((record) => {
      if (lifecycleFilter !== "ALL" && record.lifecycleStatus !== lifecycleFilter) return false;
      if (enabledFilter === "ENABLED" && !record.enabled) return false;
      if (enabledFilter === "DISABLED" && record.enabled) return false;
      if (validationFilter !== "ALL" && record.validationStatus !== validationFilter) return false;
      if (!normalizedQuery) return true;

      return [record.definitionCode, record.name, record.description]
        .some((value) => value.toLowerCase().includes(normalizedQuery));
    });
  }, [enabledFilter, lifecycleFilter, query, records, validationFilter]);
  const pagination = useTablePagination(filteredRecords);

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

  function announceAction(record: ResourceDefinitionVersionRecord, action: string) {
    const draft = action === "createDraft"
      ? records.find((item) => item.definitionCode === record.definitionCode && item.lifecycleStatus === "DRAFT")
      : null;
    const target = draft ?? record;
    navigate(`${basePath}/detail?definitionCode=${encodeURIComponent(target.definitionCode)}&definitionVersion=${encodeURIComponent(target.definitionVersion)}`);
    toast.info(pageT(`messages.${action}`, {
      name: record.name,
      version: record.definitionVersion,
    }));
  }

  function renderActions(record: ResourceDefinitionVersionRecord) {
    const stopPropagation = (event: React.SyntheticEvent) => event.stopPropagation();

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
                <DropdownMenuItem onClick={() => announceAction(record, "editDraft")}>{pageT("actions.editDraft")}</DropdownMenuItem>
                <DropdownMenuItem onClick={() => announceAction(record, "publish")}>{pageT("actions.publish")}</DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem variant="destructive" onClick={() => announceAction(record, "discardDraft")}>
                  {pageT("actions.discardDraft")}
                </DropdownMenuItem>
              </>
            ) : (
              <>
                <DropdownMenuItem onClick={() => announceAction(record, "open")}>{pageT("actions.open")}</DropdownMenuItem>
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
            <Button
              variant="outline"
              onClick={() => toast.success(pageT("messages.refreshed"))}
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
                  onValueChange={(value) => setLifecycleFilter(value as ResourceDefinitionLifecycleFilter)}
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
                  onValueChange={(value) => setEnabledFilter(value as ResourceDefinitionEnabledFilter)}
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
                  onValueChange={(value) => setValidationFilter(value as ResourceDefinitionValidationFilter)}
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
        rows={pagination.visibleItems}
        pagination={(
          <TablePagination
            itemLabel={pageT("pagination.itemLabel")}
            showRange={false}
            {...pagination.paginationProps}
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
    </div>
  );
}
