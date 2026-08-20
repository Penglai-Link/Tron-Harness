import {
  functionalUpdate,
  getCoreRowModel,
  useReactTable,
  type ColumnDef,
  type ColumnPinningState,
  type OnChangeFn,
  type PaginationState,
  type RowSelectionState,
  type SortingState,
  type Table,
} from "@tanstack/react-table";
import {
  keepPreviousData,
  useQuery,
  type QueryKey,
} from "@tanstack/react-query";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import { createSelectionColumn } from "./selectionColumn";
import type {
  ServerTableQueryContext,
  ServerTableQueryFn,
  TableColumnDef,
  TableColumnMeta,
  TablePageResult,
  TableSelectionMode,
} from "./types";

export interface UseServerTableOptions<T, TParams> {
  columns: TableColumnDef<T>[];
  enabled?: boolean;
  getRowId: (row: T, index: number, parent?: unknown) => string;
  initialPageSize?: number;
  initialSorting?: SortingState;
  onSelectionChange?: (rows: T[]) => void;
  params: TParams;
  pageSizeOptions?: readonly number[];
  queryFn: ServerTableQueryFn<T, TParams>;
  queryKey?: QueryKey;
  resource: string;
  selectionMode?: TableSelectionMode;
}

export interface ServerTableState<T> {
  data: TablePageResult<T>;
  error: Error | null;
  isFetching: boolean;
  isLoading: boolean;
  pagination: PaginationState;
  pageSizeOptions: readonly number[];
  refresh: () => Promise<unknown>;
  rowSelection: RowSelectionState;
  setPagination: OnChangeFn<PaginationState>;
  setSorting: OnChangeFn<SortingState>;
  sorting: SortingState;
  table: Table<T>;
}

function toError(error: unknown) {
  return error instanceof Error ? error : new Error("Table query failed");
}

export function useServerTable<T, TParams>({
  columns,
  enabled = true,
  getRowId,
  initialPageSize = 15,
  initialSorting = [],
  onSelectionChange,
  params,
  pageSizeOptions = [15, 25, 50, 100],
  queryFn,
  queryKey,
  resource,
  selectionMode = "none",
}: UseServerTableOptions<T, TParams>): ServerTableState<T> {
  const { t } = useTranslation("common");
  const [pagination, setPaginationState] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: initialPageSize,
  });
  const [sorting, setSortingState] = useState<SortingState>(initialSorting);
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const onSelectionChangeRef = useRef(onSelectionChange);
  onSelectionChangeRef.current = onSelectionChange;
  const paramsKey = useMemo(() => JSON.stringify(params), [params]);
  const previousParamsKeyRef = useRef(paramsKey);
  const paramsChanged = previousParamsKeyRef.current !== paramsKey;
  const queryPagination = paramsChanged ? { ...pagination, pageIndex: 0 } : pagination;
  const baseQueryKey = useMemo<QueryKey>(
    () => queryKey ?? [resource],
    [queryKey, resource],
  );
  const fullQueryKey = useMemo<QueryKey>(
    () => [...baseQueryKey, params, queryPagination, sorting],
    [baseQueryKey, params, queryPagination, sorting],
  );

  useEffect(() => {
    previousParamsKeyRef.current = paramsKey;
    setPaginationState((current) => (
      current.pageIndex === 0 ? current : { ...current, pageIndex: 0 }
    ));
    setRowSelection({});
  }, [paramsKey]);

  const query = useQuery({
    enabled,
    placeholderData: keepPreviousData,
    queryKey: fullQueryKey,
    queryFn: () => queryFn({
      pageIndex: queryPagination.pageIndex,
      pageSize: queryPagination.pageSize,
      params,
      sorting,
    }),
  });

  const setPagination = useCallback<OnChangeFn<PaginationState>>((updater) => {
    setPaginationState((current) => functionalUpdate(updater, current));
    setRowSelection({});
  }, []);

  const setSorting = useCallback<OnChangeFn<SortingState>>((updater) => {
    setSortingState((current) => functionalUpdate(updater, current));
    setPaginationState((current) => (
      current.pageIndex === 0 ? current : { ...current, pageIndex: 0 }
    ));
  }, []);

  const handleRowSelectionChange = useCallback<OnChangeFn<RowSelectionState>>((updater) => {
    setRowSelection((current) => functionalUpdate(updater, current));
  }, []);

  const selectionColumn = useMemo(
    () => selectionMode === "none" ? null : createSelectionColumn<T>(selectionMode, {
      allRows: t("table.selectAllRows"),
      row: t("table.selectRow"),
    }),
    [selectionMode, t],
  );
  const tableColumns = useMemo<ColumnDef<T, unknown>[]>(
    () => selectionColumn ? [selectionColumn, ...columns] : columns,
    [columns, selectionColumn],
  );
  const columnPinning = useMemo<ColumnPinningState>(() => ({
    left: tableColumns
      .filter((column) => (column.meta as TableColumnMeta | undefined)?.fixed === "left")
      .map((column) => column.id ?? ""),
    right: tableColumns
      .filter((column) => (column.meta as TableColumnMeta | undefined)?.fixed === "right")
      .map((column) => column.id ?? ""),
  }), [tableColumns]);
  const table = useReactTable({
    columns: tableColumns,
    data: query.data?.records ?? [],
    enableMultiRowSelection: selectionMode === "multiple",
    enableRowSelection: selectionMode !== "none",
    getCoreRowModel: getCoreRowModel(),
    getRowId,
    manualPagination: true,
    manualSorting: true,
    onPaginationChange: setPagination,
    onRowSelectionChange: handleRowSelectionChange,
    onSortingChange: setSorting,
    pageCount: Math.max(1, Math.ceil((query.data?.total ?? 0) / queryPagination.pageSize)),
    rowCount: query.data?.total ?? 0,
    state: { columnPinning, pagination: queryPagination, rowSelection, sorting },
  });

  useEffect(() => {
    onSelectionChangeRef.current?.(table.getSelectedRowModel().rows.map((row) => row.original));
  }, [query.data, rowSelection, table]);

  return {
    data: query.data ?? { records: [], total: 0 },
    error: query.error ? toError(query.error) : null,
    isFetching: query.isFetching,
    isLoading: query.isLoading,
    pageSizeOptions,
    pagination: queryPagination,
    refresh: query.refetch,
    rowSelection,
    setPagination,
    setSorting,
    sorting,
    table,
  };
}
