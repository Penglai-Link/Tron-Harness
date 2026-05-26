import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { cn } from "@/shared/lib/utils";

export interface DataTableColumn<T> {
  align?: "left" | "right";
  cellClassName?: string;
  header: string;
  headerClassName?: string;
  id: string;
  render: (row: T) => ReactNode;
  width?: string;
}

interface DataTableProps<T> {
  className?: string;
  columns: DataTableColumn<T>[];
  emptyState?: ReactNode;
  onRowClick?: (row: T, index: number) => void;
  pagination: ReactNode;
  rowKey: (row: T, index: number) => string;
  rowClassName?: (row: T) => string | undefined;
  rows: T[];
  title?: string;
  toolbar?: ReactNode;
  variant?: "default" | "embedded";
}

export function DataTable<T>({
  className,
  columns,
  emptyState,
  onRowClick,
  pagination,
  rowKey,
  rowClassName,
  rows,
  title,
  toolbar,
  variant = "default",
}: DataTableProps<T>) {
  const { t } = useTranslation("common");
  const hasHeader = title || toolbar;

  return (
    <section
      className={cn(
        "overflow-hidden",
        variant === "default"
          ? "rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)]"
          : "rounded-none border-0 bg-transparent",
        className,
      )}
    >
      {hasHeader && (
        <div
          className={cn(
            "flex flex-wrap items-center gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4",
            title && toolbar ? "justify-between" : "justify-start",
          )}
        >
          {title ? <h2 className="text-base font-semibold">{title}</h2> : null}
          {toolbar ? (
            <div
              className={cn(
                "flex min-w-0 flex-wrap items-center gap-2",
                title ? "justify-end" : "w-full justify-start",
              )}
            >
              {toolbar}
            </div>
          ) : null}
        </div>
      )}
      <div className="overflow-x-auto">
        <table className="w-full min-w-[720px] border-collapse text-left text-sm">
          {columns.some((column) => column.width) ? (
            <colgroup>
              {columns.map((column) => (
                <col key={column.id} style={column.width ? { width: column.width } : undefined} />
              ))}
            </colgroup>
          ) : null}
          <thead>
            <tr className="border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
              {columns.map((column) => (
                <th
                  className={cn(
                    "px-5 py-3",
                    column.align === "right" ? "text-right" : "text-left",
                    column.headerClassName,
                  )}
                  key={column.id}
                >
                  {column.header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.length ? (
              rows.map((row, index) => (
                <tr
                  className={cn(
                    "border-b border-[var(--color-outline-variant)] last:border-0 hover:bg-[var(--color-hover-surface)]",
                    onRowClick ? "cursor-pointer" : null,
                    rowClassName?.(row),
                  )}
                  key={rowKey(row, index)}
                  tabIndex={onRowClick ? 0 : undefined}
                  onClick={onRowClick ? () => onRowClick(row, index) : undefined}
                  onKeyDown={onRowClick
                    ? (event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        onRowClick(row, index);
                      }
                    }
                    : undefined}
                >
                  {columns.map((column) => (
                    <td
                      className={cn(
                        "px-5 py-3 align-middle",
                        column.align === "right" ? "text-right" : "text-left",
                        column.cellClassName,
                      )}
                      key={column.id}
                    >
                      {column.render(row)}
                    </td>
                  ))}
                </tr>
              ))
            ) : (
              <tr>
                <td className="px-5 py-8 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={columns.length}>
                  {emptyState ?? t("table.empty")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] px-5 py-3 text-[13px] text-[var(--color-on-surface-variant)]">
        {pagination}
      </div>
    </section>
  );
}
