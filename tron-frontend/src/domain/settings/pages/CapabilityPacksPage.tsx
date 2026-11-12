import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { LoaderCircle, PackageSearch, RefreshCw, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { SearchField } from "@/components/search-field/SearchField";
import { TablePagination } from "@/components/table-pagination";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useLocaleFormatters } from "@/shared/hooks/useLocaleFormatters";
import { useCapabilityCenter } from "../hooks/useCapabilityCenter";
import { CapabilityCenterList } from "../components/CapabilityCenterList";
import { CapabilityReleaseDialog } from "../components/CapabilityReleaseDialog";
import type { CenterPack, CenterStatus } from "../api/capabilityCenterApi";

export function CapabilityPacksPage() {
  const { t } = useTranslation("pages");
  const { formatNumber } = useLocaleFormatters();
  const text = (key: string) => t(`capabilityCenter.${key}`);
  const {
    query,
    filters,
    search,
    page,
    categories,
    mutation,
    execute,
    refresh,
  } = useCapabilityCenter();
  const [selected, setSelected] = useState<CenterPack | null>(null);
  const [remove, setRemove] = useState<CenterPack | null>(null);
  const [pendingImport, setPendingImport] = useState<File | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const busy = mutation.isPending;
  const data = page.data;
  const records = data?.records ?? [];
  const total = data?.total ?? 0;
  const openDetail = (pack: CenterPack) => {
    mutation.reset();
    setSelected(pack);
  };

  return (
    <div data-theme-part="capability-packs-page" className="min-w-0 space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap gap-1" aria-label={text("view")}>
          {(["all", "installed", "updatable"] as const).map((status) => (
            <Button
              key={status}
              size="sm"
              variant={
                query.status === status ||
                (status === "installed" &&
                  ["enabled", "disabled"].includes(query.status))
                  ? "secondary"
                  : "ghost"
              }
              aria-pressed={
                query.status === status ||
                (status === "installed" &&
                  ["enabled", "disabled"].includes(query.status))
              }
              onClick={() => filters({ status })}
            >
              {text(status === "all" ? "discover" : status)}
            </Button>
          ))}
        </div>
        <div className="flex gap-2">
          <input
            ref={input}
            type="file"
            accept=".zip,application/zip"
            aria-label={text("import")}
            hidden
            disabled={busy}
            onChange={(event) => {
              mutation.reset();
              setPendingImport(event.target.files?.[0] ?? null);
              event.target.value = "";
            }}
          />
          <Button
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={() => input.current?.click()}
          >
            <Upload size={14} aria-hidden="true" />
            {text("import")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={page.isFetching || busy}
            onClick={() => void refresh()}
          >
            <RefreshCw size={14} aria-hidden="true" />
            {text("refresh")}
          </Button>
        </div>
      </div>
      <div className="grid min-w-0 gap-2 sm:grid-cols-[minmax(0,1fr)_10rem_10rem]">
        <SearchField
          value={query.keyword}
          onChange={search}
          ariaLabel={text("search")}
          placeholder={text("search")}
          className="min-w-0"
        />
        <SelectInput
          aria-label={text("source")}
          value={query.source}
          onValueChange={(v) => filters({ source: String(v ?? "") })}
          className="w-full"
        >
          <SelectItem value="">{text("allSources")}</SelectItem>
          <SelectItem value="official">{text("sources.official")}</SelectItem>
          <SelectItem value="third_party">
            {text("sources.third_party")}
          </SelectItem>
        </SelectInput>
        <SelectInput
          aria-label={text("status")}
          value={query.status}
          onValueChange={(v) => {
            if (v) filters({ status: v as CenterStatus });
          }}
          className="w-full"
        >
          {(
            ["all", "installed", "updatable", "enabled", "disabled"] as const
          ).map((s) => (
            <SelectItem key={s} value={s}>
              {text(s === "all" ? "allStatuses" : s)}
            </SelectItem>
          ))}
        </SelectInput>
      </div>
      <div className="grid min-w-0 gap-5 xl:grid-cols-[9rem_minmax(0,1fr)]">
        <nav
          aria-label={text("categories")}
          className="flex min-w-0 flex-wrap content-start gap-1 xl:flex-col xl:items-stretch"
        >
          <Button
            variant={!query.category ? "secondary" : "ghost"}
            size="sm"
            className="justify-start"
            aria-pressed={!query.category}
            onClick={() => filters({ category: "" })}
          >
            {text("allCategories")}
          </Button>
          {categories.data?.categories.map((category) => (
            <Button
              key={category}
              variant={query.category === category ? "secondary" : "ghost"}
              size="sm"
              className="h-auto min-h-8 justify-start whitespace-normal break-words text-left"
              aria-pressed={query.category === category}
              onClick={() => filters({ category })}
            >
              {category}
            </Button>
          ))}
          {categories.isError && (
            <Button
              variant="ghost"
              size="sm"
              className="h-auto whitespace-normal"
              onClick={() => void categories.refetch()}
            >
              {text("retryCategories")}
            </Button>
          )}
        </nav>
        <section
          className="min-w-0"
          aria-label={text("results")}
          aria-busy={page.isFetching}
        >
          {data?.updateCheckIncomplete && (
            <p
              role="status"
              className="mb-4 text-sm text-[var(--color-warning)]"
            >
              {text("incomplete")}
            </p>
          )}
          {page.isError ? (
            <div role="alert" className="space-y-3 py-8">
              <p className="text-sm text-[var(--color-error)]">
                {text("loadFailed")}: {page.error.message}
              </p>
              <Button variant="outline" onClick={() => void page.refetch()}>
                {text("retry")}
              </Button>
            </div>
          ) : page.isLoading ? (
            <div
              role="status"
              className="flex items-center gap-2 py-16 text-sm text-[var(--color-on-surface-variant)]"
            >
              <LoaderCircle
                size={16}
                className="animate-spin motion-reduce:animate-none"
              />
              {text("loading")}
            </div>
          ) : records.length === 0 ? (
            <div className="flex flex-col items-center gap-3 py-16 text-center">
              <PackageSearch
                size={28}
                className="text-[var(--color-on-surface-variant)]"
                aria-hidden="true"
              />
              <p className="text-sm">
                {text(query.status === "updatable" ? "noUpdates" : "empty")}
              </p>
              {(query.keyword || query.category || query.source) && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    filters({ keyword: "", category: "", source: "" })
                  }
                >
                  {text("clearFilters")}
                </Button>
              )}
            </div>
          ) : (
            <CapabilityCenterList
              records={records}
              busy={busy}
              onDetail={openDetail}
              onToggle={(pack) => void execute({ kind: "toggle", pack })}
              onRemove={(pack) => {
                mutation.reset();
                setRemove(pack);
              }}
            />
          )}
          {!page.isError && data && (
            <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] pt-4 text-xs text-[var(--color-on-surface-variant)]">
              <span>
                {t("capabilityCenter.range", {
                  start: formatNumber(
                    total ? (query.current - 1) * query.size + 1 : 0
                  ),
                  end: formatNumber(
                    Math.min(query.current * query.size, total)
                  ),
                  total: formatNumber(total),
                })}
              </span>
              <div className="min-w-0 flex-1">
                <TablePagination
                  page={query.current}
                  pageSize={query.size}
                  pageSizeOptions={[10, 20, 50]}
                  pageCount={Math.max(1, Math.ceil(total / query.size))}
                  totalItems={total}
                  startItem={total ? (query.current - 1) * query.size + 1 : 0}
                  endItem={Math.min(query.current * query.size, total)}
                  showRange={false}
                  onPageChange={
                    page.isFetching
                      ? undefined
                      : (current) => filters({ current })
                  }
                  onPageSizeChange={(size) => filters({ size })}
                />
              </div>
            </div>
          )}
        </section>
      </div>
      {selected && (
        <CapabilityReleaseDialog
          key={selected.packId}
          pack={selected}
          busy={busy}
          error={mutation.error}
          onReviewChange={() => mutation.reset()}
          onClose={() => setSelected(null)}
          onInstall={(releaseId, installedVersion) => {
            void execute({ kind: "install", releaseId, installedVersion }).then(
              (ok) => {
                if (ok) setSelected(null);
              }
            );
          }}
        />
      )}
      <ConfirmDialog
        open={!!remove}
        title={text("uninstallTitle")}
        description={`${remove?.name ?? ""} — ${text("uninstallNotice")}${
          mutation.error ? `\n${mutation.error.message}` : ""
        }`}
        pending={busy}
        cancelText={text("cancel")}
        confirmText={text("uninstall")}
        pendingText={text("working")}
        onCancel={() => setRemove(null)}
        onConfirm={() => {
          if (remove)
            void execute({ kind: "uninstall", pack: remove }).then((ok) => {
              if (ok) setRemove(null);
            });
        }}
      />
      <ConfirmDialog
        open={!!pendingImport}
        title={text("import")}
        tone="default"
        description={`${pendingImport?.name ?? ""} — ${text("installNotice")}${
          mutation.error ? `\n${mutation.error.message}` : ""
        }`}
        pending={busy}
        cancelText={text("cancel")}
        confirmText={text("confirmImport")}
        pendingText={text("working")}
        onCancel={() => setPendingImport(null)}
        onConfirm={() => {
          if (pendingImport)
            void execute({ kind: "import", file: pendingImport }).then((ok) => {
              if (ok) setPendingImport(null);
            });
        }}
      />
    </div>
  );
}
