import type {
  ColumnDef,
  RowData,
  SortingState,
} from "@tanstack/react-table";

export interface TablePageResult<T> {
  records: T[];
  total: number;
  current?: number;
  size?: number;
}

export interface ServerTableQueryContext<TParams> {
  params: TParams;
  pageIndex: number;
  pageSize: number;
  sorting: SortingState;
}

export type ServerTableQueryFn<T, TParams> = (
  context: ServerTableQueryContext<TParams>,
) => Promise<TablePageResult<T>>;

export interface TableColumnMeta {
  align?: "left" | "right";
  cellClassName?: string;
  colClassName?: string;
  fixed?: "left" | "right";
  headerClassName?: string;
  width?: string;
}

export type TableColumnDef<T extends RowData> = ColumnDef<T, unknown> & {
  meta?: TableColumnMeta;
};

export type TableSelectionMode = "none" | "single" | "multiple";
