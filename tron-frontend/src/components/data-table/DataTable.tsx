import { Fragment, type ReactNode } from "react";
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
  header: ReactNode;
  ariaSort?: "ascending" | "descending" | "none";
  headerClassName?: string;
  id: string;
  render: (row: T) => ReactNode;
  width?: string;
}

interface DataTableProps<T> {
  className?: string;
  columns: DataTableColumn<T>[];
  emptyState?: ReactNode;
  expandedRowClassName?: string;
  isRowSelected?: (row: T, index: number) => boolean;
  isRowExpanded?: (row: T, index: number) => boolean;
  onRowClick?: (row: T, index: number) => void;
  pagination: ReactNode;
  rowKey: (row: T, index: number) => string;
  rowClassName?: (row: T) => string | undefined;
  renderExpandedRow?: (row: T, index: number) => ReactNode;
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
  expandedRowClassName,
  isRowExpanded,
  isRowSelected,
  onRowClick,
  pagination,
  rowKey,
  rowClassName,
  renderExpandedRow,
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
          ? "rounded-[var(--tron-card-radius,8px)] border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)]"
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
                aria-sort={column.ariaSort}
                className={cn(
                  "px-[var(--tron-table-cell-padding-x,1.25rem)] py-[var(--tron-table-cell-padding-y,0.75rem)]",
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
            rows.map((row, index) => {
              const expanded = isRowExpanded?.(row, index) ?? false;
              return (
                <Fragment key={rowKey(row, index)}>
                  <TableRow
                    aria-expanded={onRowClick ? expanded : undefined}
                    aria-selected={isRowSelected?.(row, index)}
                    data-state={isRowSelected?.(row, index) ? "selected" : undefined}
                    className={cn(
                      onRowClick ? "cursor-pointer" : null,
                      rowClassName?.(row),
                    )}
                    tabIndex={onRowClick ? 0 : undefined}
                    onClick={onRowClick ? (event) => {
                      if (!(event.target as HTMLElement).closest("button,a,input,select,textarea,[role=button]")) onRowClick(row, index);
                    } : undefined}
                    onKeyDown={
                      onRowClick
                        ? (event) => {
                            if (event.target !== event.currentTarget) return;
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
                          "px-[var(--tron-table-cell-padding-x,1.25rem)] py-[var(--tron-table-cell-padding-y,0.75rem)] align-middle",
                          column.align === "right" ? "text-right" : "text-left",
                          column.cellClassName,
                        )}
                        key={column.id}
                      >
                        {column.render(row)}
                      </TableCell>
                    ))}
                  </TableRow>
                  {expanded && renderExpandedRow ? (
                    <TableRow className={expandedRowClassName}>
                      <TableCell className="px-[var(--tron-table-cell-padding-x,1.25rem)] py-[var(--tron-table-cell-padding-y,0.75rem)]" colSpan={columns.length}>
                        {renderExpandedRow(row, index)}
                      </TableCell>
                    </TableRow>
                  ) : null}
                </Fragment>
              );
            })
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
      {pagination != null && pagination !== false ? (
        <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] px-5 py-3 text-[13px] text-[var(--color-on-surface-variant)]">
          {pagination}
        </div>
      ) : null}
    </section>
  );
}
