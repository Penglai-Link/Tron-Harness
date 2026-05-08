import {
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";

export interface TablePaginationState {
  endItem: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number) => void;
  page: number;
  pageCount: number;
  pageSize: number;
  startItem: number;
  totalItems: number;
}

export function useTablePagination<T>(items: readonly T[], initialPageSize = 15) {
  const [page, setPageValue] = useState(1);
  const [pageSize, setPageSizeValue] = useState(initialPageSize);
  const pageCount = Math.max(1, Math.ceil(items.length / pageSize));
  const currentPage = Math.min(page, pageCount);
  const startIndex = items.length ? (currentPage - 1) * pageSize : 0;
  const endIndex = Math.min(startIndex + pageSize, items.length);

  useEffect(() => {
    if (page > pageCount) {
      setPageValue(pageCount);
    }
  }, [page, pageCount]);

  const setPage = useCallback((value: number) => {
    setPageValue(Math.min(Math.max(value, 1), pageCount));
  }, [pageCount]);

  const setPageSize = useCallback((value: number) => {
    setPageSizeValue(value);
    setPageValue(1);
  }, []);

  const visibleItems = useMemo(
    () => items.slice(startIndex, endIndex),
    [endIndex, items, startIndex],
  );

  const paginationProps: TablePaginationState = {
    endItem: endIndex,
    onPageChange: setPage,
    onPageSizeChange: setPageSize,
    page: currentPage,
    pageCount,
    pageSize,
    startItem: items.length ? startIndex + 1 : 0,
    totalItems: items.length,
  };

  return { paginationProps, visibleItems };
}
