import {
  flexRender,
  type Column,
  type RowData,
  type Table,
} from "@tanstack/react-table";
import type { ReactNode } from "react";
import type { CSSProperties } from "react";
import { useTranslation } from "react-i18next";

import {
  Table as TablePrimitive,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { cn } from "@/shared/lib/utils";

import type { TableColumnMeta } from "./types";

export interface DataTableProps<T extends RowData> {
  className?: string;
  emptyState?: ReactNode;
  error?: Error | null;
  loading?: boolean;
  onRowClick?: (row: T, index: number) => void;
  pagination?: ReactNode;
  table: Table<T>;
  tableClassName?: string;
  title?: string;
  toolbar?: ReactNode;
  variant?: "default" | "embedded";
}

function columnMeta<T extends RowData>(table: Table<T>, columnId: string) {
  return table.getColumn(columnId)?.columnDef.meta as TableColumnMeta | undefined;
}

function fixedColumnStyle<T extends RowData>(column: Column<T, unknown>, header = false): CSSProperties | undefined {
  const fixed = (column.columnDef.meta as TableColumnMeta | undefined)?.fixed;
  if (!fixed) return undefined;

  return {
    backgroundColor: "var(--color-elevated-surface)",
    boxShadow: fixed === "left" ? "2px 0 4px rgb(0 0 0 / 6%)" : "-2px 0 4px rgb(0 0 0 / 6%)",
    left: fixed === "left" ? `${column.getStart("left")}px` : undefined,
    position: "sticky",
    right: fixed === "right" ? `${column.getAfter("right")}px` : undefined,
    zIndex: header ? 3 : 2,
  };
}

export function DataTable<T extends RowData>({
  className,
  emptyState,
  error,
  loading = false,
  onRowClick,
  pagination,
  table,
  tableClassName,
  title,
  toolbar,
  variant = "default",
}: DataTableProps<T>) {
  const { t } = useTranslation("common");
  const headerGroups = table.getHeaderGroups();
  const rows = table.getRowModel().rows;
  const leafColumns = table.getVisibleLeafColumns();
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
      {hasHeader ? (
        <div className={cn(
          "flex flex-wrap items-center gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4",
          title && toolbar ? "justify-between" : "justify-start",
        )}>
          {title ? <h2 className="text-base font-semibold">{title}</h2> : null}
          {toolbar ? <div className={cn("flex min-w-0 flex-wrap items-center gap-[12px]", title ? "justify-end" : "w-full justify-start")}>{toolbar}</div> : null}
        </div>
      ) : null}
      <TablePrimitive className={cn(tableClassName)}>
        {table.getAllLeafColumns().some((column) => (column.columnDef.meta as TableColumnMeta | undefined)?.width) ? (
          <colgroup>
            {table.getAllLeafColumns().map((column) => {
              const meta = column.columnDef.meta as TableColumnMeta | undefined;
              return <col className={meta?.colClassName} key={column.id} style={meta?.width ? { width: meta.width } : undefined} />;
            })}
          </colgroup>
        ) : null}
        <TableHeader>
          {headerGroups.map((headerGroup) => (
            <TableRow key={headerGroup.id}>
              {headerGroup.headers.map((header) => {
                const meta = columnMeta(table, header.column.id);
                return (
                  <TableHead
                    className={cn("px-5 py-3", meta?.align === "right" ? "text-right" : "text-left", meta?.headerClassName)}
                    colSpan={header.colSpan}
                    key={header.id}
                    style={fixedColumnStyle(header.column, true)}
                  >
                    {header.isPlaceholder ? null : flexRender(header.column.columnDef.header, header.getContext())}
                  </TableHead>
                );
              })}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {loading && rows.length === 0 ? (
            <TableRow><TableCell className="px-5 py-8 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={leafColumns.length}>{t("table.loading")}</TableCell></TableRow>
          ) : error && rows.length === 0 ? (
            <TableRow><TableCell className="px-5 py-8 text-center text-sm text-[var(--color-error)]" colSpan={leafColumns.length}>{error.message}</TableCell></TableRow>
          ) : rows.length ? (
            rows.map((row, index) => (
              <TableRow
                className={cn(onRowClick ? "cursor-pointer" : null)}
                data-state={row.getIsSelected() ? "selected" : undefined}
                key={row.id}
                tabIndex={onRowClick ? 0 : undefined}
                onClick={(event) => {
                  if (onRowClick && !(event.target as HTMLElement).closest("[data-table-selection]")) onRowClick(row.original, index);
                }}
                onKeyDown={(event) => {
                  if (onRowClick && (event.key === "Enter" || event.key === " ")) {
                    event.preventDefault();
                    onRowClick(row.original, index);
                  }
                }}
              >
                {row.getVisibleCells().map((cell) => {
                  const meta = columnMeta(table, cell.column.id);
                  return <TableCell className={cn("px-5 py-3 align-middle", meta?.align === "right" ? "text-right" : "text-left", meta?.cellClassName)} key={cell.id} style={fixedColumnStyle(cell.column)}>{flexRender(cell.column.columnDef.cell, cell.getContext())}</TableCell>;
                })}
              </TableRow>
            ))
          ) : (
            <TableRow><TableCell className="px-5 py-8 text-center text-sm text-[var(--color-on-surface-variant)]" colSpan={leafColumns.length}>{emptyState ?? t("table.empty")}</TableCell></TableRow>
          )}
        </TableBody>
      </TablePrimitive>
      {pagination ? <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] px-5 py-3 text-[13px] text-[var(--color-on-surface-variant)]">{pagination}</div> : null}
    </section>
  );
}
