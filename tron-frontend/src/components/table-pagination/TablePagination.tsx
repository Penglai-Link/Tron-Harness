import { ChevronLeft, ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useLocaleFormatters } from "@/shared/hooks/useLocaleFormatters";
import { Button } from "@/components/ui/button";
import {
  SelectInput,
  TextInput,
} from "@/components/forms";
import { SelectItem } from "@/components/ui/select";

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
  showRange?: boolean;
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
  showRange = true,
  startItem,
  totalItems,
}: TablePaginationProps) {
  const { t } = useTranslation("common");
  const { formatNumber } = useLocaleFormatters();
  const canGoPrevious = Boolean(onPageChange) && page > 1;
  const canGoNext = Boolean(onPageChange) && page < pageCount;
  const startLabel = formatNumber(startItem);
  const endLabel = formatNumber(endItem);
  const totalLabel = formatNumber(totalItems);
  const pageCountLabel = formatNumber(pageCount);

  return (
    <div className={`flex w-full flex-wrap items-center gap-3 ${showRange ? "justify-between" : "justify-end"}`}>
      {showRange ? (
        <span>
          {totalItems
            ? t("pagination.range", {
              defaultValue: "{{start}}-{{end}} of {{total}} {{itemLabel}} visible",
              end: endLabel,
              itemLabel,
              start: startLabel,
              total: totalLabel,
            })
            : formatNumber(0)}
        </span>
      ) : null}
      <div className="flex flex-wrap items-center gap-[12px]">
        <label className="flex items-center gap-[12px]">
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">{t("pagination.rows")}</span>
          <SelectInput
            aria-label={t("pagination.rowsPerPage")}
            className="h-9 w-20"
            disabled={!onPageSizeChange}
            value={pageSize}
            onValueChange={(event) => onPageSizeChange?.(Number(event))}
          >
            {pageSizeOptions.map((item) => <SelectItem key={item} value={item}>{formatNumber(item)}</SelectItem>)}
          </SelectInput>
        </label>
        <Button size="icon" variant="secondary" aria-label={t("pagination.previousPage")} disabled={!canGoPrevious} onClick={() => onPageChange?.(page - 1)}>
          <ChevronLeft size={14} aria-hidden="true" />
        </Button>
        <label className="flex items-center gap-[12px]">
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">{t("pagination.page")}</span>
          <TextInput
            aria-label={t("pagination.goToPage")}
            className="h-9 w-16 text-center"
            disabled={!onPageChange}
            max={pageCount}
            min={1}
            type="number"
            value={page}
            onChange={(event) => onPageChange?.(Number(event.target.value) || 1)}
          />
          <span className="text-[12px] text-[var(--color-on-surface-variant)]">{t("pagination.of")} {pageCountLabel}</span>
        </label>
        <Button size="icon" variant="secondary" aria-label={t("pagination.nextPage")} disabled={!canGoNext} onClick={() => onPageChange?.(page + 1)}>
          <ChevronRight size={14} aria-hidden="true" />
        </Button>
      </div>
    </div>
  );
}
