import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { cn } from "@/shared/lib/utils";

export interface DataTableColumn<T> {
  align?: "left" | "right";
  cellClassName?: string;
  colClassName?: string;
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
  tableClassName?: string;
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
  tableClassName,
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
          ? "rounded-[8px] border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)]"
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
                "flex min-w-0 flex-wrap items-center gap-[12px]",
                title ? "justify-end" : "w-full justify-start",
              )}
            >
              {toolbar}
            </div>
          ) : null}
        </div>
      )}
      <Table className={cn(tableClassName)}>
        {columns.some((column) => column.width) ? (
          <colgroup>
            {columns.map((column) => (
              <col
                className={column.colClassName}
                key={column.id}
                style={column.width ? { width: column.width } : undefined}
              />
            ))}
          </colgroup>
        ) : null}
        <TableHeader>
          <TableRow>
            {columns.map((column) => (
              <TableHead
                className={cn(
                  "px-5 py-3",
                  column.align === "right" ? "text-right" : "text-left",
                  column.headerClassName,
                )}
                key={column.id}
              >
                {column.header}
              </TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.length ? (
            rows.map((row, index) => (
              <TableRow
                className={cn(
                  onRowClick ? "cursor-pointer" : null,
                  rowClassName?.(row),
                )}
                key={rowKey(row, index)}
                tabIndex={onRowClick ? 0 : undefined}
                onClick={onRowClick ? () => onRowClick(row, index) : undefined}
                onKeyDown={
                  onRowClick
                    ? (event) => {
                        if (event.key === "Enter" || event.key === " ") {
                          event.preventDefault();
                          onRowClick(row, index);
                        }
                      }
                    : undefined
                }
              >
                {columns.map((column) => (
                  <TableCell
                    className={cn(
                      "px-5 py-3 align-middle",
                      column.align === "right" ? "text-right" : "text-left",
                      column.cellClassName,
                    )}
                    key={column.id}
                  >
                    {column.render(row)}
                  </TableCell>
                ))}
              </TableRow>
            ))
          ) : (
            <TableRow>
              <TableCell
                className="px-5 py-8 text-center text-sm text-[var(--color-on-surface-variant)]"
                colSpan={columns.length}
              >
                {emptyState ?? t("table.empty")}
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] px-5 py-3 text-[13px] text-[var(--color-on-surface-variant)]">
        {pagination}
      </div>
    </section>
  );
}
