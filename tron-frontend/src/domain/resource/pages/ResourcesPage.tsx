import {
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  AlertCircle,
  Database,
  FileArchive,
  Filter,
  FolderOpen,
  Image,
  MoreHorizontal,
  RefreshCcw,
  Settings2,
  SlidersHorizontal,
  Upload,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/components/ui/button";
import {
  DataTable,
  MetricCard,
  PageHeader,
  SearchField,
  SelectInput,
  TablePagination,
  useLocaleFormatters,
  useTablePagination,
} from "@/shared";

import { useResourceList } from "@/domain/resource/hooks";
import type { ResourceRecord } from "@/domain/resource/model";
import {
  getStatusTone,
  resourceStatusFilters,
  resourceTypeFilters,
} from "@/domain/resource/constants";
import { SelectItem } from "@/components/ui/select";

function ResourceIcon({ resource }: { resource: ResourceRecord }) {
  const iconClassName = "size-4";
  if (resource.type.includes("PostgreSQL")) return <Database className={iconClassName} aria-hidden="true" />;
  if (resource.type.includes("S3")) return <FolderOpen className={iconClassName} aria-hidden="true" />;
  if (resource.type.includes("JSON")) return <Settings2 className={iconClassName} aria-hidden="true" />;
  if (resource.type.includes("Media")) return <Image className={iconClassName} aria-hidden="true" />;
  return <AlertCircle className={iconClassName} aria-hidden="true" />;
}

export function ResourcesPage() {
  const { t } = useTranslation(["pages", "common"]);
  const { formatNumber } = useLocaleFormatters();
  const resources = useResourceList();
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const [typeFilter, setTypeFilter] = useState<(typeof resourceTypeFilters)[number]>("All");
  const [statusFilter, setStatusFilter] = useState<(typeof resourceStatusFilters)[number] | "All">("All");
  const [rowMenu, setRowMenu] = useState<string | null>(null);
  const [notice, setNotice] = useState("");
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:resources.${key}`, { defaultValue, ...options })
  );

  const filtered = useMemo(() => resources.filter((resource) => {
    const normalizedQuery = query.trim().toLowerCase();
    const matchesSearch = !normalizedQuery
      || resource.name.toLowerCase().includes(normalizedQuery)
      || resource.type.toLowerCase().includes(normalizedQuery)
      || resource.location.toLowerCase().includes(normalizedQuery);
    const matchesType = typeFilter === "All" || resource.type === typeFilter;
    const matchesStatus = statusFilter === "All" || resource.status === statusFilter;
    return matchesSearch && matchesType && matchesStatus;
  }), [query, resources, statusFilter, typeFilter]);

  const resourcePagination = useTablePagination(filtered);
  const visibleResources = resourcePagination.visibleItems;
  const allSelected = visibleResources.length > 0 && visibleResources.every((resource) => selected.includes(resource.id));
  const activeSources = resources.filter((resource) => resource.status === "Active").length;

  function toggleSelected(id: string) {
    setSelected((current) => (current.includes(id) ? current.filter((item) => item !== id) : [...current, id]));
  }

  function toggleAllSelected() {
    setSelected((current) => (
      allSelected
        ? current.filter((id) => !visibleResources.some((resource) => resource.id === id))
        : Array.from(new Set([...current, ...visibleResources.map((resource) => resource.id)]))
    ));
    setNotice(allSelected ? pageT("messages.selectionCleared", "Visible resources cleared") : pageT("messages.selectionApplied", "Visible resources selected"));
  }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] flex-col">
      <PageHeader
        title={pageT("title", "Resources")}
        actions={(
          <>
            <Button size="lg" variant="outline" onClick={() => setNotice(pageT("messages.bulkActions", "{{count}} resources queued for bulk actions", { count: formatNumber(selected.length) }))}>
              {pageT("actions.bulk", "Bulk Actions")}
            </Button>
            <Button size="lg" onClick={() => setNotice(pageT("messages.importOpened", "Import Resource drawer opened"))}>
              {pageT("actions.import", "Import Resource")}
            </Button>
          </>
        )}
      />

      <section className="grid gap-5 xl:grid-cols-3 mb-6" aria-label={pageT("metrics.ariaLabel", "Resource metrics")}>
        <MetricCard
          icon={<Database size={18} aria-hidden="true" />}
          label={pageT("metrics.activeSources.label", "Active Data Sources")}
          value={formatNumber(activeSources)}
          trend={pageT("metrics.activeSources.trend", "{{count}} currently match the active governance state", { count: formatNumber(activeSources) })}
        />
        <MetricCard
          icon={<FileArchive size={18} aria-hidden="true" />}
          label={pageT("metrics.managedFiles.label", "Managed Files")}
          value={formatNumber(resources.length)}
          trend={pageT("metrics.managedFiles.trend", "Tracked across connected knowledge, storage, and dataset endpoints.")}
        />
        <MetricCard
          icon={<RefreshCcw size={18} aria-hidden="true" />}
          label={pageT("metrics.totalStorage.label", "Total Storage")}
          value={pageT("metrics.totalStorage.value", "8.4 TB")}
          trend={pageT("metrics.totalStorage.trend", "72% of allocated capacity")}
        />
      </section>

      <DataTable<ResourceRecord>
        rows={visibleResources}
        pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "resources")} {...resourcePagination.paginationProps} />}
        rowKey={(row) => row.id}
        toolbar={(
          <>
            <div className="min-w-[240px] flex-1 sm:max-w-[360px]">
              <SearchField
                ariaLabel={pageT("search.ariaLabel", "Search resources")}
                clearLabel={pageT("search.clear", "Clear resource search")}
                placeholder={pageT("search.placeholder", "Search resources...")}
                value={query}
                onChange={(value) => {
                  setQuery(value);
                  setNotice(value ? pageT("messages.searchFiltered", "Search filtered to {{value}}", { value }) : pageT("messages.searchCleared", "Search cleared"));
                }}
              />
            </div>
            <SelectInput
              aria-label={pageT("filters.typeAriaLabel", "Filter resources by type")}
              className="w-full sm:w-52"
              value={typeFilter}
              onValueChange={(event) => {
                setTypeFilter(event as (typeof resourceTypeFilters)[number]);
                setNotice(pageT("messages.typeFilter", "Type filter changed to {{value}}", { value: event }));
              }}
            >
              {resourceTypeFilters.map((item) => (
                <SelectItem key={item} value={item}>
                  {item === "All" ? pageT("filters.allTypes", "All Types") : item}
                </SelectItem>
              ))}
            </SelectInput>
            <SelectInput
              aria-label={pageT("filters.statusAriaLabel", "Filter resources by status")}
              className="w-full sm:w-44"
              value={statusFilter}
              onValueChange={(event) => {
                setStatusFilter(event as (typeof resourceStatusFilters)[number] | "All");
                setNotice(pageT("messages.statusFilter", "Status filter changed to {{value}}", { value: event }));
              }}
            >
              {resourceStatusFilters.map((item) => (
                <SelectItem key={item} value={item}>
                  {item}
                </SelectItem>
              ))}
              <SelectItem value="All">{pageT("filters.allStatuses", "All Statuses")}</SelectItem>
            </SelectInput>
            <Button size="lg" type="button" variant="default" onClick={toggleAllSelected}>
              {allSelected ? pageT("actions.clearVisible", "Clear Visible") : pageT("actions.selectVisible", "Select Visible")}
            </Button>
            <Button size="icon-lg" variant="ghost" aria-label={pageT("actions.advancedFilters", "Open advanced resource filters")} onClick={() => setNotice(pageT("messages.advancedFilters", "Advanced filters opened"))}>
              <Filter size={18} aria-hidden="true" />
            </Button>
          </>
        )}
        columns={[
          {
            id: "selected",
            header: "",
            headerClassName: "w-14",
            cellClassName: "w-14",
            render: (row) => (
              <input
                className="size-4 rounded border-[var(--color-outline-variant)]"
                checked={selected.includes(row.id)}
                aria-label={pageT("aria.selectRow", "Select {{name}}", { name: row.name })}
                type="checkbox"
                onChange={() => toggleSelected(row.id)}
              />
            ),
          },
          {
            id: "name",
            header: pageT("columns.name", "Resource Name"),
            render: (row) => (
              <div className="flex items-center gap-3">
                <ResourceIcon resource={row} />
                <span className="font-medium">{row.name}</span>
              </div>
            ),
          },
          {
            id: "type",
            header: pageT("columns.type", "Type"),
            render: (row) => <span className="text-[var(--color-on-surface-variant)]">{row.type}</span>,
          },
          {
            id: "status",
            header: pageT("columns.status", "Status"),
            render: (row) => <Badge tone={getStatusTone(row.status)}>{row.status}</Badge>,
          },
          {
            id: "location",
            header: pageT("columns.location", "Location"),
            render: (row) => <span className="text-[var(--color-on-surface-variant)]">{row.location}</span>,
          },
          {
            id: "lastAccessed",
            header: pageT("columns.lastAccessed", "Last Accessed"),
            align: "right",
            render: (row) => <span className="text-[var(--color-on-surface-variant)]">{row.lastAccessed}</span>,
          },
          {
            id: "actions",
            header: "",
            align: "right",
            headerClassName: "w-12",
            cellClassName: "relative w-12",
            render: (row) => (
              <div className="relative text-right">
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label={pageT("aria.openActions", "Open actions for {{name}}", { name: row.name })}
                  onClick={() => setRowMenu((current) => (current === row.id ? null : row.id))}
                >
                  <MoreHorizontal size={16} aria-hidden="true" />
                </Button>
                {rowMenu === row.id ? (
                  <div className="absolute right-4 top-12 z-20 w-40 rounded-lg border border-[var(--color-outline-variant)] bg-white p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.12)]">
                    <button className="block w-full rounded-md px-3 py-2 text-left hover:bg-[var(--color-surface-container-low)]" type="button" onClick={() => setNotice(pageT("messages.openDetails", "{{name}} details opened", { name: row.name }))}>
                      {pageT("actions.openDetails", "Open details")}
                    </button>
                    <button className="block w-full rounded-md px-3 py-2 text-left hover:bg-[var(--color-surface-container-low)]" type="button" onClick={() => setNotice(pageT("messages.reviewPolicy", "{{name}} policy review queued", { name: row.name }))}>
                      {pageT("actions.reviewPolicy", "Review policy")}
                    </button>
                  </div>
                ) : null}
              </div>
            ),
          },
        ]}
        emptyState={pageT("emptyState", "No resources match the current filters.")}
      />

      <div className="sr-only" aria-live="polite">
        {notice}
      </div>
    </div>
  );
}
