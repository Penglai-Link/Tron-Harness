import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight } from "lucide-react";

import { fetchToolDomains, listTools } from "@/domain/agent/api";
import { listToolDomainsMock, listToolsMock } from "@/domain/agent/mock";
import type { PlatformToolRecord, ToolCatalogDomainRecord } from "@/domain/agent/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { DataTable } from "@/components/data-table";
import { SearchField } from "@/components/search-field";
import { TablePagination } from "@/components/table-pagination";
import {
  PageHeader,
  useTablePagination,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

export function PlatformToolsPage() {
  const { t } = useTranslation(["pages", "common"]);
  const toast = useToast();
  const dataSource = getServiceDataSource("agent");
  const [domains, setDomains] = useState<ToolCatalogDomainRecord[]>([]);
  const [tools, setTools] = useState<PlatformToolRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [domainSearch, setDomainSearch] = useState("");
  const [debouncedDomainSearch, setDebouncedDomainSearch] = useState("");
  const [expandedDomainCodes, setExpandedDomainCodes] = useState<Set<string>>(() => new Set());

  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:settingsTools.${key}`, { defaultValue, ...options })
  );

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedDomainSearch(domainSearch.trim().toLowerCase());
    }, 260);

    return () => window.clearTimeout(timer);
  }, [domainSearch]);

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

  const domainPagination = useTablePagination(visibleDomains);
  const toolsByDomain = useMemo(() => {
    const grouped = new Map<string, PlatformToolRecord[]>();
    for (const tool of tools) {
      const domainTools = grouped.get(tool.domainCode) ?? [];
      domainTools.push(tool);
      grouped.set(tool.domainCode, domainTools);
    }
    return grouped;
  }, [tools]);

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

  function toggleDomain(domainCode: string) {
    setExpandedDomainCodes((current) => {
      const next = new Set(current);
      if (next.has(domainCode)) {
        next.delete(domainCode);
      } else {
        next.add(domainCode);
      }
      return next;
    });
  }

  return (
    <div className="space-y-6">
      <PageHeader title={pageT("title", "Platform Tools")} />

      <div className="space-y-4">
        <DataTable<ToolCatalogDomainRecord>
          rowKey={(row) => row.domainCode}
          rows={domainPagination.visibleItems}
          tableClassName="[&_td]:py-2 [&_th]:py-2"
          expandedRowClassName="bg-[var(--color-surface-container)]"
          isRowExpanded={(row) => expandedDomainCodes.has(row.domainCode)}
          onRowClick={(row) => toggleDomain(row.domainCode)}
          pagination={
            <TablePagination
              itemLabel={pageT("domains.itemLabel", "tool domains")}
              {...domainPagination.paginationProps}
            />
          }
          renderExpandedRow={(domain) => {
            const domainTools = toolsByDomain.get(domain.domainCode) ?? [];
            return domainTools.length ? (
              <div className="overflow-x-auto rounded-md border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)]">
                <table className="w-full min-w-[680px] text-left text-sm">
                  <thead className="border-b border-[var(--color-card-border)] text-xs font-medium text-[var(--color-on-surface-variant)]">
                    <tr>
                      <th className="px-4 py-2.5">{pageT("domains.tools.columns.name", "工具名称")}</th>
                      <th className="px-4 py-2.5">{pageT("domains.tools.columns.englishName", "工具名称英文")}</th>
                      <th className="px-4 py-2.5">{pageT("domains.tools.columns.description", "描述")}</th>
                      <th className="px-4 py-2.5">{pageT("domains.tools.columns.type", "类型")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {domainTools.map((tool) => (
                      <tr className="border-b border-[var(--color-card-border)] last:border-b-0" key={tool.toolCode}>
                        <td className="px-4 py-2.5 font-medium text-[var(--color-on-surface)]">{tool.name}</td>
                        <td className="px-4 py-2.5 font-mono text-xs text-[var(--color-on-surface-variant)]">{tool.toolName}</td>
                        <td className="px-4 py-2.5 text-[13px] text-[var(--color-on-surface-variant)]">{tool.description}</td>
                        <td className="px-4 py-2.5">{t(`common:toolOperationKind.${tool.operationKind}`, { defaultValue: tool.operationKind })}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : <p className="py-2 text-sm text-[var(--color-on-surface-variant)]">{pageT("domains.tools.empty", "该领域没有可用工具。")}</p>;
          }}
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
              render: (row) => <span className="flex items-center gap-2 font-medium text-[var(--color-on-surface)]"><ChevronRight className={expandedDomainCodes.has(row.domainCode) ? "rotate-90 transition-transform" : "transition-transform"} size={16} aria-hidden="true" />{row.domainCode}</span>,
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

      </div>
    </div>
  );
}
