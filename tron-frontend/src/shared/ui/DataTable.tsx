import type { ReactNode } from "react";

import { TablePagination } from "@/shared/ui/TablePagination";
import { cn } from "@/shared/lib/utils";

export interface DataTableColumn<T> {
  align?: "left" | "right";
  header: string;
  id: string;
  render: (row: T) => ReactNode;
}

interface DataTableProps<T> {
  columns: DataTableColumn<T>[];
  emptyState?: ReactNode;
  pagination?: ReactNode;
  rowKey: (row: T, index: number) => string;
  rows: T[];
  title?: string;
  toolbar?: ReactNode;
}

export function DataTable<T>({ columns, emptyState, pagination, rowKey, rows, title, toolbar }: DataTableProps<T>) {
  return (
    <section className="overflow-hidden rounded-2xl border border-[var(--color-card-border)] bg-white">
      {(title || toolbar) && (
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
          {title ? <h2 className="text-base font-semibold">{title}</h2> : <span />}
          {toolbar ? <div className="flex flex-wrap items-center gap-2">{toolbar}</div> : null}
        </div>
      )}
      <div className="overflow-x-auto">
        <table className="w-full min-w-[720px] border-collapse text-left text-sm">
          <thead>
            <tr className="border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
              {columns.map((column) => (
                <th className={cn("px-5 py-3", column.align === "right" ? "text-right" : "text-left")} key={column.id}>
                  {column.header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.length ? (
              rows.map((row, index) => (
                <tr className="border-b border-[var(--color-outline-variant)] last:border-0 hover:bg-[var(--color-hover-surface)]" key={rowKey(row, index)}>
                  {columns.map((column) => (
                    <td className={cn("px-5 py-3 align-middle", column.align === "right" ? "text-right" : "text-left")} key={column.id}>
                      {column.render(row)}
                    </td>
                  ))}
                </tr>
              ))
            ) : (
              <tr>
                <td className="px-5 py-8 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={columns.length}>
                  {emptyState ?? "No records match the current filters."}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] px-5 py-3 text-[13px] text-[var(--color-on-surface-variant)]">
        {pagination ?? (
          <TablePagination
            endItem={rows.length}
            page={1}
            pageCount={1}
            pageSize={15}
            startItem={rows.length ? 1 : 0}
            totalItems={rows.length}
          />
        )}
      </div>
    </section>
  );
}
