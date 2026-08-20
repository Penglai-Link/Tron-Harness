// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

import { useServerTable } from "./useServerTable";

interface RecordItem {
  id: string;
  name: string;
}

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return function QueryWrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
}

describe("useServerTable", () => {
  it("requests again when external params or pagination change", async () => {
    const queryFn = vi.fn(async ({ pageIndex, pageSize, params }: {
      pageIndex: number;
      pageSize: number;
      params: { search: string };
    }) => ({
      records: [{ id: `${params.search}-${pageIndex}`, name: params.search }],
      total: 30,
    }));
    const { result, rerender } = renderHook(
      ({ search }) => useServerTable({
        columns: [{ accessorKey: "name", header: "Name" }],
        getRowId: (row: RecordItem) => row.id,
        params: { search },
        queryFn,
        resource: "records",
      }),
      {
        initialProps: { search: "first" },
        wrapper: createWrapper(),
      },
    );

    await waitFor(() => expect(queryFn).toHaveBeenCalledWith(expect.objectContaining({
      pageIndex: 0,
      pageSize: 15,
      params: { search: "first" },
    })));
    await waitFor(() => expect(result.current.data.total).toBe(30));

    act(() => result.current.table.nextPage());

    await waitFor(() => expect(queryFn).toHaveBeenCalledWith(expect.objectContaining({
      pageIndex: 1,
      params: { search: "first" },
    })));

    rerender({ search: "second" });

    await waitFor(() => expect(queryFn).toHaveBeenCalledWith(expect.objectContaining({
      pageIndex: 0,
      params: { search: "second" },
    })));
  });

  it("selects all rows loaded for the current page", async () => {
    const onSelectionChange = vi.fn();
    const { result } = renderHook(
      () => useServerTable({
        columns: [{ accessorKey: "name", header: "Name" }],
        getRowId: (row: RecordItem) => row.id,
        onSelectionChange,
        params: {},
        queryFn: async () => ({
          records: [
            { id: "one", name: "One" },
            { id: "two", name: "Two" },
          ],
          total: 2,
        }),
        resource: "records",
        selectionMode: "multiple",
      }),
      { wrapper: createWrapper() },
    );

    await waitFor(() => expect(result.current.data.records).toHaveLength(2));

    act(() => result.current.table.toggleAllPageRowsSelected());

    await waitFor(() => expect(onSelectionChange).toHaveBeenLastCalledWith([
      { id: "one", name: "One" },
      { id: "two", name: "Two" },
    ]));
  });

  it("pins columns configured with fixed metadata", async () => {
    const { result } = renderHook(
      () => useServerTable({
        columns: [
          { accessorKey: "name", header: "Name" },
          {
            cell: () => "Actions",
            header: "Actions",
            id: "actions",
            meta: { fixed: "right", width: "7rem" },
            size: 112,
          },
        ],
        getRowId: (row: RecordItem) => row.id,
        params: {},
        queryFn: async () => ({ records: [{ id: "one", name: "One" }], total: 1 }),
        resource: "records",
      }),
      { wrapper: createWrapper() },
    );

    await waitFor(() => expect(result.current.data.records).toHaveLength(1));

    expect(result.current.table.getColumn("actions")?.getIsPinned()).toBe("right");
  });
});
