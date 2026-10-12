import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import type { PlatformToolRecord, ToolCatalogDomainRecord } from "@/domain/agent/model";
import { DataTable } from "@/components/data-table";
import { SearchField } from "@/components/search-field";
import { TablePagination } from "@/components/table-pagination";
import {
  PageHeader,
  useTablePagination,
} from "@/shared";
import { usePlatformToolSettings } from "../hooks/usePlatformToolSettings";
import { Button } from "@/components/ui/button";
import { ToolPermissionControl } from "../components/ToolPermissionControl";

export function PlatformToolsPage() {
  const { t } = useTranslation(["pages", "common"]);
  const { domains, tools, loading, saving, loaded, permissions, dirty, refreshTools, savePermissions, discard, setPermission } = usePlatformToolSettings();
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
      <PageHeader title={pageT("title", "Platform Tools")} actions={<div className="flex flex-wrap items-center gap-2">
        <span role="status" className="text-xs text-[var(--color-on-surface-variant)]">{loading ? pageT("permissions.loading", "Loading tool permissions…") : dirty ? pageT("permissions.dirty", "Unsaved changes") : ""}</span>
        <Button variant="outline" disabled={loading || saving || dirty} onClick={() => void refreshTools()}>{pageT("permissions.refresh", "Refresh")}</Button>
        <Button variant="outline" disabled={!dirty || saving || loading} onClick={discard}>{pageT("permissions.discard", "Discard changes")}</Button>
        <Button disabled={!dirty || !loaded || loading || saving} onClick={() => void savePermissions()}>{pageT(saving ? "permissions.saving" : "permissions.save", saving ? "Saving…" : "Save")}</Button>
      </div>} />

        <DataTable<ToolCatalogDomainRecord>
          rowKey={(row) => row.domainCode}
          rows={domainPagination.visibleItems}
          tableClassName="table-fixed [&_td]:py-2 [&_th]:py-2"
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
                <table className="w-full min-w-[760px] table-fixed whitespace-normal text-left text-sm [&_td]:break-words">
                  <colgroup>
                    <col className="w-[190px]" />
                    <col />
                    <col className="w-[64px]" />
                    <col className="w-[290px]" />
                  </colgroup>
                  <thead className="border-b border-[var(--color-card-border)] text-xs font-medium text-[var(--color-on-surface-variant)]">
                    <tr>
                      <th className="px-4 py-2.5">{pageT("domains.tools.columns.name", "工具")}</th>
                      <th className="px-4 py-2.5">{pageT("domains.tools.columns.description", "描述")}</th>
                      <th className="px-4 py-2.5">{pageT("domains.tools.columns.type", "类型")}</th>
                      <th className="px-4 py-2.5">{pageT("permissions.title", "Execution permission")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {domainTools.map((tool) => (
                      <tr className="border-b border-[var(--color-card-border)] last:border-b-0" key={tool.toolCode}>
                        <td className="px-4 py-2.5">
                          <p className="font-medium text-[var(--color-on-surface)]">{tool.name}</p>
                          <p className="mt-1 font-mono text-xs text-[var(--color-on-surface-variant)]">{tool.toolName}</p>
                        </td>
                        <td className="px-4 py-2.5 text-[13px] text-[var(--color-on-surface-variant)]">{tool.description}</td>
                        <td className="px-4 py-2.5">{t(`common:toolOperationKind.${tool.operationKind}`, { defaultValue: tool.operationKind })}</td>
                        <td className="px-4 py-2.5"><ToolPermissionControl name={tool.name} disabled={loading || saving || !loaded}
                          value={permissions.find((item) => item.toolCode === tool.toolCode) ?? { toolCode: tool.toolCode, permission: "run", confirmLevel: "low" }}
                          onChange={setPermission} /></td>
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
              render: (row) => <Button variant="ghost" aria-expanded={expandedDomainCodes.has(row.domainCode)} aria-label={`${row.domainCode}: ${row.name}`} onClick={(event) => { event.stopPropagation(); toggleDomain(row.domainCode); }} className="flex items-center gap-2 font-medium text-[var(--color-on-surface)]">{row.domainCode}</Button>,
            },
            {
              id: "description",
              header: pageT("domains.columns.description", "Description"),
              width: "56%",
              cellClassName: "whitespace-normal",
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
          ]}
        />
    </div>
  );
}
