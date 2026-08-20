import {
  useEffect,
  useRef,
  type ChangeEvent,
} from "react";
import type { RowData } from "@tanstack/react-table";

import type { TableColumnDef, TableSelectionMode } from "./types";

function SelectionCheckbox({
  checked,
  indeterminate = false,
  label,
  onChange,
}: {
  checked: boolean;
  indeterminate?: boolean;
  label: string;
  onChange: (event: ChangeEvent<HTMLInputElement>) => void;
}) {
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (ref.current) ref.current.indeterminate = indeterminate;
  }, [indeterminate]);

  return (
    <input
      ref={ref}
      aria-label={label}
      checked={checked}
      className="size-4 accent-[var(--color-primary)]"
      data-table-selection="true"
      type="checkbox"
      onChange={onChange}
      onClick={(event) => event.stopPropagation()}
    />
  );
}

export function createSelectionColumn<T extends RowData>(
  selectionMode: Exclude<TableSelectionMode, "none">,
  labels: { allRows: string; row: string },
): TableColumnDef<T> {
  return {
    cell: ({ row }) => (
      <SelectionCheckbox
        checked={row.getIsSelected()}
        label={labels.row}
        onChange={row.getToggleSelectedHandler()}
      />
    ),
    enableSorting: false,
    header: ({ table }) => (
      selectionMode === "multiple" ? (
        <SelectionCheckbox
          checked={table.getIsAllPageRowsSelected()}
          indeterminate={table.getIsSomePageRowsSelected()}
          label={labels.allRows}
          onChange={table.getToggleAllPageRowsSelectedHandler()}
        />
      ) : null
    ),
    id: "__selection",
    meta: {
      cellClassName: "w-12 px-3",
      headerClassName: "w-12 px-3",
      width: "3rem",
    },
  };
}
