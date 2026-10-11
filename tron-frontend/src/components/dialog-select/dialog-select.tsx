"use client";

import { Check, ChevronLeft, ChevronRight, X, ChevronsUpDown, Search } from "lucide-react";

import * as React from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

export interface DialogSelectOption {
  description?: string;
  disabled?: boolean;
  label: string;
  value: string;
}

export interface DialogSelectProps {
  "aria-describedby"?: string;
  ariaLabel: string;
  cancelText?: string;
  className?: string;
  confirmText?: string;
  disabled?: boolean;
  emptyMessage?: string;
  id?: string;
  itemLabel?: string;
  loading?: boolean;
  loadingMessage?: string;
  multiple?: boolean;
  onChange: (values: string[]) => void;
  onPageChange?: (page: number) => void;
  options: DialogSelectOption[];
  page?: number;
  pageCount?: number;
  pagination?: boolean;
  placeholder?: string;
  removeLabel?: string;
  searchEmptyMessage?: string;
  searchPlaceholder?: string;
  selectedEmptyMessage?: string;
  selectedLabel?: string;
  selectedValues: string[];
  title?: string;
  totalItems?: number;
  triggerClassName?: string;
}

const PAGE_SIZE = 10;

const stateMessageClassName =
  "flex w-full items-center rounded-[4px] border border-input bg-background px-3 py-2 text-sm text-muted-foreground";

export function DialogSelect(props: DialogSelectProps) {
  const { t } = useTranslation("common");

  const {
    "aria-describedby": ariaDescribedBy,
    ariaLabel,
    cancelText = t("dialogSelect.cancel", "Cancel"),
    className,
    confirmText = t("dialogSelect.confirm", "OK"),
    disabled = false,
    emptyMessage = t("dialogSelect.noOptions", "No options available."),
    id,
    itemLabel = t("dialogSelect.items", "items"),
    loading = false,
    loadingMessage = t("dialogSelect.loading", "Loading options..."),
    multiple = false,
    onChange,
    onPageChange,
    options,
    page,
    pageCount,
    pagination = false,
    placeholder = t("dialogSelect.placeholder", "Select..."),
    removeLabel = t("dialogSelect.remove", "Remove"),
    searchEmptyMessage = t("dialogSelect.searchEmpty", "No matching options."),
    searchPlaceholder = t("dialogSelect.search", "Search options..."),
    selectedEmptyMessage = t("dialogSelect.selectedEmpty", "No items selected."),
    selectedLabel = t("dialogSelect.selected", "Selected"),
    selectedValues,
    title,
    totalItems,
    triggerClassName,
  } = props;

  const [open, setOpen] = React.useState(false);
  const [pendingValues, setPendingValues] = React.useState<string[]>([]);
  const [searchTerm, setSearchTerm] = React.useState("");
  const [internalPage, setInternalPage] = React.useState(1);

  const uniqueSelectedValues = React.useMemo(
    () => Array.from(new Set(selectedValues.filter(Boolean))),
    [selectedValues]
  );

  const optionMap = React.useMemo(
    () => new Map(options.map((option) => [option.value, option])),
    [options]
  );

  const triggerOptions = React.useMemo(
    () =>
      uniqueSelectedValues.map(
        (value) => optionMap.get(value) ?? { label: value, value }
      ),
    [optionMap, uniqueSelectedValues]
  );

  const pendingUniqueValues = React.useMemo(
    () => Array.from(new Set(pendingValues.filter(Boolean))),
    [pendingValues]
  );

  const pendingItems = React.useMemo(
    () =>
      pendingUniqueValues.map(
        (value) => optionMap.get(value) ?? { label: value, value }
      ),
    [optionMap, pendingUniqueValues]
  );

  const normalizedSearch = searchTerm.trim().toLowerCase();
  const filteredOptions = React.useMemo(() => {
    if (!normalizedSearch) return options;
    return options.filter(
      (option) =>
        option.label.toLowerCase().includes(normalizedSearch) ||
        (option.description?.toLowerCase().includes(normalizedSearch) ?? false)
    );
  }, [normalizedSearch, options]);

  const uncontrolledPageCount = Math.max(1, Math.ceil(filteredOptions.length / PAGE_SIZE));
  const controlledPagination = pagination && page !== undefined && pageCount !== undefined;
  const resolvedPageCount = controlledPagination
    ? Math.max(1, pageCount ?? 1)
    : uncontrolledPageCount;
  const resolvedPage = Math.min(
    Math.max(1, controlledPagination ? (page ?? 1) : internalPage),
    resolvedPageCount
  );
  const displayedTotalItems = totalItems ?? filteredOptions.length;
  const pageItems = React.useMemo(() => {
    if (controlledPagination) return filteredOptions;
    return filteredOptions.slice(
      (resolvedPage - 1) * PAGE_SIZE,
      resolvedPage * PAGE_SIZE
    );
  }, [controlledPagination, filteredOptions, resolvedPage]);

  React.useEffect(() => {
    setInternalPage((current) => Math.min(current, uncontrolledPageCount));
  }, [uncontrolledPageCount]);

  const handleOpenChange = (nextOpen: boolean) => {
    if (nextOpen) {
      setPendingValues(uniqueSelectedValues);
      setSearchTerm("");
      setInternalPage(1);
    }
    setOpen(nextOpen);
  };

  const toggleOption = (option: DialogSelectOption) => {
    if (option.disabled) return;
    if (!multiple) {
      setPendingValues((current) =>
        current[0] === option.value ? current : [option.value]
      );
      return;
    }
    setPendingValues((current) =>
      current.includes(option.value)
        ? current.filter((value) => value !== option.value)
        : [...current, option.value]
    );
  };

  const removeValue = (value: string) => {
    setPendingValues((current) => current.filter((item) => item !== value));
  };

  const handleConfirm = () => {
    onChange(pendingUniqueValues);
    setOpen(false);
  };

  if (loading && !open) {
    return (
      <div className={cn(stateMessageClassName, className)}>
        {loadingMessage}
      </div>
    );
  }

  if (!options.length && !open) {
    return (
      <div className={cn(stateMessageClassName, className)}>
        {emptyMessage}
      </div>
    );
  }

  return (
    <div className={cn("w-full", className)}>
      <Dialog onOpenChange={handleOpenChange} open={open}>
        <DialogTrigger
          aria-describedby={ariaDescribedBy}
          aria-label={ariaLabel}
          disabled={disabled}
          id={id}
          render={
            <button
              className={cn(
                "relative flex min-h-9 w-full flex-wrap items-center gap-1 rounded-[4px] border border-input bg-background py-1 pl-2 pr-8 text-left text-sm outline-none transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring/25 disabled:cursor-not-allowed disabled:opacity-50",
                triggerClassName
              )}
              type="button"
            />
          }
        >
          {triggerOptions.length === 0 ? (
            <span className="px-1 text-muted-foreground">{placeholder}</span>
          ) : multiple ? (
            triggerOptions.map((item) => (
              <span
                className="flex items-center rounded-[3px] bg-accent px-2 py-0.5 text-xs font-medium text-accent-foreground"
                key={item.value}
              >
                {item.label}
              </span>
            ))
          ) : (
            <span className="min-w-0 flex-1 truncate px-1">
              {triggerOptions[0].label}
            </span>
          )}
          <ChevronsUpDown
            aria-hidden="true"
            className="absolute right-2 top-1/2 size-4 -translate-y-1/2 opacity-60"
          />
        </DialogTrigger>
        <DialogContent
          className="grid w-[min(96vw,60rem)] max-w-none gap-0 p-0 sm:max-w-none"
          showCloseButton={false}
        >
          <div className="flex items-center justify-between gap-3 border-b px-5 py-4">
            <DialogTitle>{title ?? ariaLabel}</DialogTitle>
            <DialogClose
              render={
                <Button aria-label={t("common:actions.close")} size="icon-sm" variant="ghost" />
              }
            >
              <X aria-hidden="true" size={16} />
            </DialogClose>
          </div>

          <div className="grid h-[min(70vh,34rem)] grid-cols-1 grid-rows-[1fr_1fr] md:grid-cols-2 md:grid-rows-1">
            <div className="flex min-h-0 flex-col border-b md:border-r">
              <div className="flex h-14 shrink-0 items-center px-3">
                <div className="relative w-full">
                  <Search
                    aria-hidden="true"
                    className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
                  />
                  <Input
                    aria-label={ariaLabel}
                    className="pl-8"
                    placeholder={searchPlaceholder}
                    value={searchTerm}
                    onChange={(event) => {
                      setSearchTerm(event.target.value);
                      setInternalPage(1);
                    }}
                  />
                </div>
              </div>
              <ScrollArea className="flex-1 min-h-0">
                <div className="flex flex-col gap-0.5 p-2">
                  {loading ? (
                    <p className="px-2 py-6 text-center text-sm text-muted-foreground">
                      {loadingMessage}
                    </p>
                  ) : pageItems.length === 0 ? (
                    <p className="px-2 py-6 text-center text-sm text-muted-foreground">
                      {searchEmptyMessage}
                    </p>
                  ) : (
                    pageItems.map((item) => {
                      const isSelected = pendingUniqueValues.includes(item.value);
                      return (
                        <button
                          aria-pressed={multiple ? isSelected : undefined}
                          className="flex w-full items-start gap-2.5 rounded-[4px] px-2 py-1.5 text-left outline-none transition-colors hover:bg-accent focus-visible:bg-accent disabled:pointer-events-none disabled:opacity-50"
                          disabled={item.disabled}
                          key={item.value}
                          type="button"
                          onClick={() => toggleOption(item)}
                        >
                          <span
                            aria-hidden="true"
                            className={cn(
                              "mt-0.5 flex size-4 shrink-0 items-center justify-center border transition-colors",
                              multiple ? "rounded-[3px]" : "rounded-full",
                              isSelected
                                ? "border-primary bg-primary text-primary-foreground"
                                : "border-input bg-background"
                            )}
                          >
                            {isSelected ? (
                              multiple ? (
                                <Check aria-hidden="true" className="size-3" strokeWidth={3} />
                              ) : (
                                <span className="size-1.5 rounded-full bg-primary-foreground" />
                              )
                            ) : null}
                          </span>
                          <span className="flex min-w-0 flex-1 flex-col">
                            <span className="truncate text-sm text-foreground">
                              {item.label}
                            </span>
                            {item.description ? (
                              <span className="truncate text-xs text-muted-foreground">
                                {item.description}
                              </span>
                            ) : null}
                          </span>
                        </button>
                      );
                    })
                  )}
                </div>
              </ScrollArea>
              {pagination ? (
                <div className="flex items-center justify-between gap-2 px-3 py-2">
                  <span className="text-xs text-muted-foreground">
                    {displayedTotalItems} {itemLabel}
                  </span>
                  <div className="flex items-center gap-1">
                    <Button
                      aria-label={t("pagination.previousPage")}
                      disabled={resolvedPage <= 1}
                      size="icon-sm"
                      variant="ghost"
                      onClick={() => {
                        if (controlledPagination) {
                          onPageChange?.(resolvedPage - 1);
                        } else {
                          setInternalPage(resolvedPage - 1);
                        }
                      }}
                    >
                      <ChevronLeft aria-hidden="true" size={14} />
                    </Button>
                    <span className="min-w-12 text-center text-xs tabular-nums text-muted-foreground">
                      {resolvedPage} / {resolvedPageCount}
                    </span>
                    <Button
                      aria-label={t("pagination.nextPage")}
                      disabled={resolvedPage >= resolvedPageCount}
                      size="icon-sm"
                      variant="ghost"
                      onClick={() => {
                        if (controlledPagination) {
                          onPageChange?.(resolvedPage + 1);
                        } else {
                          setInternalPage(resolvedPage + 1);
                        }
                      }}
                    >
                      <ChevronRight aria-hidden="true" size={14} />
                    </Button>
                  </div>
                </div>
              ) : null}
            </div>

            <div className="flex min-h-0 flex-col border-b">
              <div className="flex h-14 shrink-0 items-center px-4">
                <span className="text-sm font-medium text-foreground">
                  {selectedLabel} ({pendingUniqueValues.length})
                </span>
              </div>
              <ScrollArea className="flex-1 min-h-0">
                <div className="flex flex-col gap-0.5 p-2">
                  {pendingItems.map((item) => (
                    <div
                      className="flex items-center gap-2 rounded-[4px] px-2 py-1.5"
                      key={item.value}
                    >
                      <span className="flex min-w-0 flex-1 flex-col">
                        <span className="truncate text-sm text-foreground">
                          {item.label}
                        </span>
                        {item.description ? (
                          <span className="truncate text-xs text-muted-foreground">
                            {item.description}
                          </span>
                        ) : null}
                      </span>
                      <Button
                        aria-label={`${removeLabel} ${item.label}`}
                        size="icon-sm"
                        variant="ghost"
                        onClick={() => removeValue(item.value)}
                      >
                        <X aria-hidden="true" size={14} />
                      </Button>
                    </div>
                  ))}
                  {pendingItems.length === 0 ? (
                    <p className="px-2 py-6 text-center text-sm text-muted-foreground">
                      {selectedEmptyMessage}
                    </p>
                  ) : null}
                </div>
              </ScrollArea>
            </div>
          </div>

          <div className="flex justify-end gap-[12px] px-5 py-4">
            <Button onClick={() => setOpen(false)} variant="outline">
              {cancelText}
            </Button>
            <Button onClick={handleConfirm}>{confirmText}</Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
