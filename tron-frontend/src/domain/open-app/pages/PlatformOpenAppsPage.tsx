import {
  type CSSProperties,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router";
import {
  Check,
  Copy,
  MoreHorizontal,
  Plus,
  RefreshCcw,
} from "lucide-react";

import {
  fetchPlatformOpenAppCatalog,
  fetchPlatformOpenApps,
  updatePlatformOpenApp,
} from "@/domain/open-app/api";
import {
  formatOpenAppStatusLabel,
  OpenAppMetrics,
  statusTone,
  useOpenAppPageT,
} from "@/domain/open-app/components";
import {
  listPlatformOpenAppCatalogMock,
  listPlatformOpenAppsMock,
  updatePlatformOpenAppMock,
} from "@/domain/open-app/mock";
import type {
  ManagedOpenAppSummary,
  OpenAppStatus,
} from "@/domain/open-app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Badge,
  Button,
  DataTable,
  PageHeader,
  SearchField,
  TablePagination,
  useDismissibleLayer,
  useLocaleFormatters,
  useTablePagination,
  useToast,
} from "@/shared";

type RowMenuState = {
  anchorRect: DOMRect;
  openAppCode: string;
  triggerElement: HTMLButtonElement;
};

const ACTION_MENU_GAP = 8;
const ACTION_MENU_MARGIN = 12;
const ACTION_MENU_WIDTH = 176;

function getActionMenuStyle(anchorRect: DOMRect, itemCount: number): CSSProperties {
  const estimatedMenuHeight = itemCount * 44 + 16;
  const left = Math.min(
    window.innerWidth - ACTION_MENU_WIDTH - ACTION_MENU_MARGIN,
    Math.max(ACTION_MENU_MARGIN, anchorRect.right - ACTION_MENU_WIDTH),
  );
  const top = anchorRect.bottom + ACTION_MENU_GAP + estimatedMenuHeight <= window.innerHeight - ACTION_MENU_MARGIN
    ? anchorRect.bottom + ACTION_MENU_GAP
    : Math.max(ACTION_MENU_MARGIN, anchorRect.top - ACTION_MENU_GAP - estimatedMenuHeight);

  return {
    left,
    position: "fixed",
    top,
    width: ACTION_MENU_WIDTH,
  };
}

export function PlatformOpenAppsPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const pageT = useOpenAppPageT();
  const dataSource = getServiceDataSource("admin");
  const [records, setRecords] = useState<ManagedOpenAppSummary[]>([]);
  const [catalogCount, setCatalogCount] = useState(0);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const [pendingStatusCode, setPendingStatusCode] = useState<string | null>(null);
  const [copiedCode, setCopiedCode] = useState<string | null>(null);
  const rowMenuRef = useRef<HTMLDivElement | null>(null);
  const copyResetTimerRef = useRef<number | null>(null);

  const filteredRecords = useMemo(() => records.filter((record) => {
    const normalizedQuery = query.trim().toLowerCase();
    if (!normalizedQuery) return true;
    return [
      record.name,
      record.openAppCode,
      record.description ?? "",
    ].some((value) => value.toLowerCase().includes(normalizedQuery));
  }), [query, records]);
  const enabledCount = records.filter((record) => record.status === "Enabled").length;
  const pagination = useTablePagination(filteredRecords);
  const openMenuRow = useMemo(
    () => (openRowMenu ? records.find((record) => record.openAppCode === openRowMenu.openAppCode) ?? null : null),
    [openRowMenu, records],
  );

  async function loadPlatformData() {
    setLoading(true);
    try {
      const [recordsResult, catalogResult] = dataSource === "api"
        ? await Promise.allSettled([
          fetchPlatformOpenApps(),
          fetchPlatformOpenAppCatalog(),
        ])
        : await Promise.allSettled([
          Promise.resolve(listPlatformOpenAppsMock()),
          Promise.resolve(listPlatformOpenAppCatalogMock()),
        ]);

      if (recordsResult.status === "fulfilled") {
        setRecords(recordsResult.value);
      } else {
        setRecords([]);
        toast.error(recordsResult.reason instanceof Error ? recordsResult.reason.message : pageT("errors.listLoad", "Platform open-apps could not be loaded"));
      }

      if (catalogResult.status === "fulfilled") {
        setCatalogCount(catalogResult.value.length);
      } else {
        setCatalogCount(0);
        toast.error(catalogResult.reason instanceof Error ? catalogResult.reason.message : pageT("errors.catalogLoad", "OpenAPI capability catalog could not be loaded"));
      }
    } finally {
      setLoading(false);
    }
  }

  async function handleCopyOpenAppCode(openAppCode: string) {
    try {
      await navigator.clipboard.writeText(openAppCode);
      setCopiedCode(openAppCode);
      if (copyResetTimerRef.current !== null) {
        window.clearTimeout(copyResetTimerRef.current);
      }
      copyResetTimerRef.current = window.setTimeout(() => {
        setCopiedCode((current) => (current === openAppCode ? null : current));
      }, 1400);
    } catch {
      toast.error(pageT("errors.copyCode", "Open App Code could not be copied"));
    }
  }

  async function handleStatusUpdate(
    row: ManagedOpenAppSummary,
    nextStatus: Exclude<OpenAppStatus, "Archived"> | "Archived",
  ) {
    setPendingStatusCode(row.openAppCode);
    setOpenRowMenu(null);
    try {
      if (dataSource === "api") {
        await updatePlatformOpenApp(row.openAppCode, {
          description: row.description,
          name: row.name,
          status: nextStatus,
        });
      } else {
        updatePlatformOpenAppMock(row.openAppCode, {
          description: row.description,
          name: row.name,
          status: nextStatus,
        });
      }
      await loadPlatformData();
      toast.success(pageT("messages.statusUpdated", "{{name}} status updated", { name: row.name }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.metadataSave", "Open-app could not be updated"));
    } finally {
      setPendingStatusCode(null);
    }
  }

  useEffect(() => {
    void loadPlatformData();
  }, [dataSource]);

  useEffect(() => () => {
    if (copyResetTimerRef.current !== null) {
      window.clearTimeout(copyResetTimerRef.current);
    }
  }, []);

  useEffect(() => {
    setOpenRowMenu(null);
  }, [query, records.length, pagination.paginationProps.page, pagination.paginationProps.pageSize]);

  useDismissibleLayer({
    onDismiss: () => setOpenRowMenu(null),
    open: openRowMenu !== null,
    popupRef: rowMenuRef,
    triggerElement: openRowMenu?.triggerElement,
  });

  return (
    <>
      <PageHeader
        title={pageT("title.list", "Platform OpenAPI")}
        actions={(
          <>
            <Button variant="secondary" disabled={loading} aria-busy={loading} onClick={() => void loadPlatformData()}>
              <RefreshCcw size={16} aria-hidden="true" />
              {loading ? pageT("actions.refreshing", "Refreshing...") : pageT("actions.refresh", "Refresh")}
            </Button>
            <Button onClick={() => navigate("/settings/openapi/new")}>
              <Plus size={16} aria-hidden="true" />
              {pageT("actions.new", "New")}
            </Button>
          </>
        )}
      />

      <OpenAppMetrics
        activeCount={enabledCount}
        catalogCount={catalogCount}
        recordCount={records.length}
      />

      <div className="mt-5">
        <DataTable<ManagedOpenAppSummary>
          title={pageT("list.title", "Registered Open Apps")}
          rows={pagination.visibleItems}
          rowKey={(row) => row.openAppCode}
          onRowClick={(row) => navigate(`/settings/openapi/detail?openAppCode=${encodeURIComponent(row.openAppCode)}`)}
          pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "open apps")} {...pagination.paginationProps} />}
          toolbar={(
            <SearchField
              ariaLabel={pageT("list.searchAria", "Search open apps")}
              className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
              placeholder={pageT("list.searchPlaceholder", "Search open-apps...")}
              value={query}
              onChange={setQuery}
            />
          )}
          emptyState={pageT("list.empty", "No open-apps match the current filters.")}
          columns={[
            {
              id: "name",
              header: pageT("list.columns.name", "App Name"),
              width: "16%",
              cellClassName: "py-4",
              render: (row) => (
                <p className="font-medium text-[var(--color-on-surface)]">{row.name}</p>
              ),
            },
            {
              id: "openAppCode",
              header: pageT("list.columns.appCode", "App Code"),
              width: "20%",
              cellClassName: "py-4",
              render: (row) => (
                <div className="flex items-center gap-[12px]">
                  <span className="font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                    {row.openAppCode}
                  </span>
                  <button
                    aria-label={pageT("aria.copyCode", "Copy app code for {{name}}", { name: row.name })}
                    className="inline-flex size-7 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]"
                    title={copiedCode === row.openAppCode
                      ? pageT("actions.copied", "Copied")
                      : pageT("actions.copyCode", "Copy app code")}
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation();
                      void handleCopyOpenAppCode(row.openAppCode);
                    }}
                  >
                    {copiedCode === row.openAppCode ? <Check size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
                  </button>
                </div>
              ),
            },
            {
              id: "description",
              header: pageT("list.columns.description", "Description"),
              width: "22%",
              render: (row) => (
                <p className="max-w-[18rem] text-sm leading-5 text-[var(--color-on-surface-variant)]">
                  {row.description || pageT("list.noDescription", "No description")}
                </p>
              ),
            },
            {
              id: "status",
              header: pageT("list.columns.status", "Status"),
              width: "10%",
              render: (row) => (
                <Badge tone={statusTone(row.status)}>
                  {formatOpenAppStatusLabel(pageT, row.status)}
                </Badge>
              ),
            },
            {
              id: "grants",
              header: pageT("list.columns.grants", "Grants"),
              width: "16%",
              render: (row) => (
                <div className="space-y-1 text-[13px] text-[var(--color-on-surface-variant)]">
                  <p>{pageT("list.userCount", "Users: {{count}}", { count: row.userGrantCount })}</p>
                  <p>{pageT("list.operationCount", "Operations: {{count}}", { count: row.capabilityGrantCount })}</p>
                </div>
              ),
            },
            {
              id: "updatedAt",
              header: pageT("list.columns.updatedAt", "Updated"),
              width: "16%",
              render: (row) => formatDateTime(row.updatedAt, {
                dateStyle: "medium",
                timeStyle: "short",
              }),
            },
            {
              id: "actions",
              header: pageT("list.columns.actions", "Actions"),
              headerClassName: "w-16",
              cellClassName: "w-16",
              align: "right",
              width: "8%",
              render: (row) => (
                <div className="flex justify-end">
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-expanded={openRowMenu?.openAppCode === row.openAppCode}
                    aria-haspopup="menu"
                    aria-label={pageT("aria.openActions", "Open actions for {{name}}", { name: row.name })}
                    disabled={pendingStatusCode === row.openAppCode}
                    onClick={(event) => {
                      event.stopPropagation();
                      if (openRowMenu?.openAppCode === row.openAppCode) {
                        setOpenRowMenu(null);
                        return;
                      }
                      setOpenRowMenu({
                        anchorRect: event.currentTarget.getBoundingClientRect(),
                        openAppCode: row.openAppCode,
                        triggerElement: event.currentTarget,
                      });
                    }}
                  >
                    <MoreHorizontal size={16} aria-hidden="true" />
                  </Button>
                </div>
              ),
            },
          ]}
        />
      </div>

      {openRowMenu && openMenuRow ? createPortal(
        <div
          ref={rowMenuRef}
          className="z-[70] rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] p-2 shadow-[0_18px_38px_rgba(15,23,42,0.18)]"
          role="menu"
          style={getActionMenuStyle(openRowMenu.anchorRect, openMenuRow.status === "Archived" ? 1 : 3)}
        >
          <button
            className="flex w-full items-center rounded-xl px-3 py-2 text-left text-sm text-[var(--color-on-surface)] transition-colors hover:bg-[var(--color-surface-container)]"
            role="menuitem"
            type="button"
            onClick={() => {
              setOpenRowMenu(null);
              navigate(`/settings/openapi/edit?openAppCode=${encodeURIComponent(openMenuRow.openAppCode)}`);
            }}
          >
            {pageT("actions.edit", "Edit")}
          </button>
          {openMenuRow.status !== "Archived" ? (
            <>
              <button
                className="flex w-full items-center rounded-xl px-3 py-2 text-left text-sm text-[var(--color-on-surface)] transition-colors hover:bg-[var(--color-surface-container)]"
                role="menuitem"
                type="button"
                onClick={() => void handleStatusUpdate(openMenuRow, "Archived")}
              >
                {pageT("actions.archive", "Archive")}
              </button>
              <button
                className="flex w-full items-center rounded-xl px-3 py-2 text-left text-sm text-[var(--color-on-surface)] transition-colors hover:bg-[var(--color-surface-container)]"
                role="menuitem"
                type="button"
                onClick={() => void handleStatusUpdate(
                  openMenuRow,
                  openMenuRow.status === "Enabled" ? "Disabled" : "Enabled",
                )}
              >
                {openMenuRow.status === "Enabled"
                  ? pageT("actions.disable", "Disable")
                  : pageT("actions.enable", "Enable")}
              </button>
            </>
          ) : null}
        </div>,
        document.body,
      ) : null}
    </>
  );
}
