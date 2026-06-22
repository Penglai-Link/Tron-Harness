import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { fetchToolDomains, listTools } from "@/domain/agent/api";
import { listToolDomainsMock, listToolsMock } from "@/domain/agent/mock";
import type { PlatformToolRecord, ToolCatalogDomainRecord } from "@/domain/agent/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  DataTable,
  PageHeader,
  SearchField,
  SelectInput,
  TablePagination,
  useTablePagination,
  useToast,
} from "@/shared";

export function PlatformToolsPage() {
  const { t } = useTranslation(["pages", "common"]);
  const toast = useToast();
  const dataSource = getServiceDataSource("agent");
  const [domains, setDomains] = useState<ToolCatalogDomainRecord[]>([]);
  const [tools, setTools] = useState<PlatformToolRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [domainSearch, setDomainSearch] = useState("");
  const [toolSearch, setToolSearch] = useState("");
  const [selectedToolDomainCode, setSelectedToolDomainCode] = useState("all");
  const [debouncedDomainSearch, setDebouncedDomainSearch] = useState("");
  const [debouncedToolSearch, setDebouncedToolSearch] = useState("");

  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:settingsTools.${key}`, { defaultValue, ...options })
  );

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedDomainSearch(domainSearch.trim().toLowerCase());
    }, 260);

    return () => window.clearTimeout(timer);
  }, [domainSearch]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedToolSearch(toolSearch.trim().toLowerCase());
    }, 260);

    return () => window.clearTimeout(timer);
  }, [toolSearch]);

  const visibleDomains = useMemo(() => {
    const normalizedSearch = debouncedDomainSearch;
    if (!normalizedSearch) {
      return domains;
    }

    return domains.filter((domain) => (
      [
        domain.domainCode,
        domain.name,
        domain.description,
        domain.sampleTools.join(" "),
      ].join(" ").toLowerCase().includes(normalizedSearch)
    ));
  }, [debouncedDomainSearch, domains]);

  const visibleTools = useMemo(() => {
    const normalizedSearch = debouncedToolSearch;
    return tools.filter((tool) => {
      if (selectedToolDomainCode !== "all" && tool.domainCode !== selectedToolDomainCode) {
        return false;
      }

      if (!normalizedSearch) {
        return true;
      }

      return [
        tool.toolCode,
        tool.domainCode,
        tool.name,
        tool.description,
        tool.operationKind,
        t(`common:toolOperationKind.${tool.operationKind}`, { defaultValue: tool.operationKind }),
      ].join(" ").toLowerCase().includes(normalizedSearch);
    });
  }, [debouncedToolSearch, selectedToolDomainCode, t, tools]);

  const domainPagination = useTablePagination(visibleDomains);
  const toolPagination = useTablePagination(visibleTools);

  async function refreshTools() {
    setLoading(true);
    try {
      const [nextDomains, nextTools] = dataSource === "api"
        ? await Promise.all([
          fetchToolDomains(),
          listTools(),
        ])
        : await Promise.all([
          listToolDomainsMock(),
          listToolsMock(),
        ]);

      setDomains(nextDomains);
      setTools(nextTools);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Platform tools could not be loaded"));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refreshTools();
  }, [dataSource]);

  useEffect(() => {
    if (selectedToolDomainCode === "all") {
      return;
    }

    const domainStillPresent = domains.some((domain) => domain.domainCode === selectedToolDomainCode);
    if (!domainStillPresent) {
      setSelectedToolDomainCode("all");
    }
  }, [domains, selectedToolDomainCode]);

  return (
    <div className="space-y-6">
      <PageHeader title={pageT("title", "Platform Tools")} />

      <div className="space-y-4">
        <DataTable<ToolCatalogDomainRecord>
          rowKey={(row) => row.domainCode}
          rows={domainPagination.visibleItems}
          tableClassName="[&_td]:py-2 [&_th]:py-2"
          pagination={
            <TablePagination
              itemLabel={pageT("domains.itemLabel", "tool domains")}
              {...domainPagination.paginationProps}
            />
          }
          toolbar={
            <SearchField
              ariaLabel={pageT("domains.searchAria", "Search tool domains")}
              className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
              disabled={loading}
              placeholder={pageT("domains.searchPlaceholder", "Search tool domains...")}
              value={domainSearch}
              onChange={setDomainSearch}
            />
          }
          title={pageT("domains.title", "Tool Domains")}
          emptyState={pageT("domains.empty", "No domains match the current filters.")}
          columns={[
            {
              id: "domain",
              header: pageT("domains.columns.domainCode", "Domain"),
              width: "18%",
              render: (row) => (
                <span className="font-medium text-[var(--color-on-surface)]">{row.domainCode}</span>
              ),
            },
            {
              id: "description",
              header: pageT("domains.columns.description", "Description"),
              width: "34%",
              render: (row) => (
                <p className="text-[13px] text-[var(--color-on-surface-variant)]">
                  {row.description}
                </p>
              ),
            },
            {
              id: "tools",
              header: pageT("domains.columns.toolCount", "Tools"),
              width: "12%",
              align: "right",
              render: (row) => <span>{row.toolCount}</span>,
            },
            {
              id: "mutation",
              header: pageT("domains.columns.mutation", "Mutation"),
              width: "14%",
              render: (row) => (
                <span>
                  {row.hasMutationTools
                    ? pageT("domains.mutationYes", "Has mutations")
                    : pageT("domains.mutationNo", "Read-only")}
                </span>
              ),
            },
            {
              id: "samples",
              header: pageT("domains.columns.samples", "Sample Tools"),
              width: "22%",
              render: (row) => {
                const sampleTools = row.sampleTools.length > 0
                  ? row.sampleTools.join(", ")
                  : pageT("domains.sampleFallback", "N/A");
                return <p className="truncate text-[12px] text-[var(--color-on-surface-variant)]">{sampleTools}</p>;
              },
            },
          ]}
        />

        <DataTable<PlatformToolRecord>
          rowKey={(row) => row.toolCode}
          rows={toolPagination.visibleItems}
          tableClassName="[&_td]:py-2 [&_th]:py-2"
          pagination={
            <TablePagination
              itemLabel={pageT("tools.itemLabel", "tools")}
              {...toolPagination.paginationProps}
            />
          }
          toolbar={
            <div className="flex w-full flex-wrap gap-3">
              <SearchField
                ariaLabel={pageT("tools.searchAria", "Search platform tools")}
                className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
                disabled={loading}
                placeholder={pageT("tools.searchPlaceholder", "Search platform tools...")}
                value={toolSearch}
                onChange={setToolSearch}
              />
              <SelectInput
                aria-label={pageT("tools.domainFilterAria", "Filter platform tools by domain")}
                className="min-w-[180px] sm:max-w-[220px]"
                disabled={loading}
                value={selectedToolDomainCode}
                onChange={(event) => {
                  setSelectedToolDomainCode(event.target.value);
                }}
              >
                <option value="all">
                  {pageT("tools.domainFilterAll", "All domains")}
                </option>
                {domains.map((domain) => (
                  <option key={domain.domainCode} value={domain.domainCode}>
                    {domain.name}
                  </option>
                ))}
              </SelectInput>
            </div>
          }
          title={pageT("tools.title", "Tools")}
          emptyState={pageT("tools.empty", "No tools match the current filters.")}
          columns={[
            {
              id: "toolCode",
              header: pageT("tools.columns.toolCode", "Tool Code"),
              width: "22%",
              render: (row) => <span className="font-mono text-[13px]">{row.toolCode}</span>,
            },
            {
              id: "domainCode",
              header: pageT("tools.columns.domainCode", "Domain"),
              width: "16%",
              render: (row) => row.domainCode,
            },
            {
              id: "name",
              header: pageT("tools.columns.name", "Name"),
              width: "16%",
              render: (row) => (
                <span className="font-medium text-[var(--color-on-surface)]">
                  {row.name}
                </span>
              ),
            },
            {
              id: "operation",
              header: pageT("tools.columns.operationKind", "Kind"),
              width: "12%",
              align: "right",
              render: (row) => t(`common:toolOperationKind.${row.operationKind}`, { defaultValue: row.operationKind }),
            },
            {
              id: "description",
              header: pageT("tools.columns.description", "Description"),
              render: (row) => (
                <p className="text-[13px] text-[var(--color-on-surface-variant)]">
                  {row.description}
                </p>
              ),
            },
          ]}
        />
      </div>
    </div>
  );
}
