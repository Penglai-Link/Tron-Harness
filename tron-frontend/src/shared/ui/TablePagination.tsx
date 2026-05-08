import {
  ChevronLeft,
  ChevronRight,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  SelectInput,
  TextInput,
} from "@/shared/ui/Forms";

const defaultPageSizeOptions = [15, 25, 50, 100] as const;

interface TablePaginationProps {
  endItem: number;
  itemLabel?: string;
  onPageChange?: (page: number) => void;
  onPageSizeChange?: (pageSize: number) => void;
  page: number;
  pageCount: number;
  pageSize: number;
  pageSizeOptions?: readonly number[];
  startItem: number;
  totalItems: number;
}

export function TablePagination({
  endItem,
  itemLabel = "records",
  onPageChange,
  onPageSizeChange,
  page,
  pageCount,
  pageSize,
  pageSizeOptions = defaultPageSizeOptions,
  startItem,
  totalItems,
}: TablePaginationProps) {
  const canGoPrevious = Boolean(onPageChange) && page > 1;
  const canGoNext = Boolean(onPageChange) && page < pageCount;

  return (
    <div className="flex w-full flex-wrap items-center justify-between gap-3">
      <span>
        {totalItems ? `${startItem}-${endItem}` : "0"} of {totalItems} {itemLabel} visible
      </span>
      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-2">
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">Rows</span>
          <SelectInput
            aria-label="Rows per page"
            className="h-9 w-20"
            disabled={!onPageSizeChange}
            value={pageSize}
            onChange={(event) => onPageSizeChange?.(Number(event.target.value))}
          >
            {pageSizeOptions.map((item) => <option key={item} value={item}>{item}</option>)}
          </SelectInput>
        </label>
        <Button size="icon" variant="secondary" aria-label="Previous page" disabled={!canGoPrevious} onClick={() => onPageChange?.(page - 1)}>
          <ChevronLeft size={14} aria-hidden="true" />
        </Button>
        <label className="flex items-center gap-2">
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">Page</span>
          <TextInput
            aria-label="Go to page"
            className="h-9 w-16 text-center"
            disabled={!onPageChange}
            max={pageCount}
            min={1}
            type="number"
            value={page}
            onChange={(event) => onPageChange?.(Number(event.target.value) || 1)}
          />
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">of {pageCount}</span>
        </label>
        <Button size="icon" variant="secondary" aria-label="Next page" disabled={!canGoNext} onClick={() => onPageChange?.(page + 1)}>
          <ChevronRight size={14} aria-hidden="true" />
        </Button>
      </div>
    </div>
  );
}
