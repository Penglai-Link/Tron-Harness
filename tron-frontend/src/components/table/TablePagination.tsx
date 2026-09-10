import type { Table } from "@tanstack/react-table";

import { TablePagination as SharedTablePagination } from "@/components/table-pagination";

export interface TablePaginationProps<T> {
  itemLabel?: string;
  pageSizeOptions?: readonly number[];
  showRange?: boolean;
  table: Table<T>;
  totalItems: number;
}

export function TablePagination<T>({
  itemLabel,
  pageSizeOptions,
  showRange = true,
  table,
  totalItems,
}: TablePaginationProps<T>) {
  const page = table.getState().pagination.pageIndex + 1;
  const pageSize = table.getState().pagination.pageSize;
  const pageCount = Math.max(1, table.getPageCount());
  const startItem = totalItems ? (page - 1) * pageSize + 1 : 0;
  const endItem = Math.min(page * pageSize, totalItems);

  return (
    <SharedTablePagination
      endItem={endItem}
      itemLabel={itemLabel}
      onPageChange={(next) => table.setPageIndex(Math.min(Math.max(next - 1, 0), pageCount - 1))}
      onPageSizeChange={(next) => table.setPageSize(next)}
      page={page}
      pageCount={pageCount}
      pageSize={pageSize}
      pageSizeOptions={pageSizeOptions}
      showRange={showRange}
      startItem={startItem}
      totalItems={totalItems}
    />
  );
}
