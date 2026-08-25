import {
  useEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
  MoreHorizontal,
  Plus,
  RefreshCcw,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  PageHeader,
  TablePagination,
  useDismissibleLayer,
  useTablePagination,
  useToast,
} from "@/shared";

import {
  deletePlatformModel,
  duplicatePlatformModel,
  disablePlatformModel,
  enablePlatformModel,
  fetchPlatformModels,
  setPlatformDefaultModel,
  verifyPlatformModel,
} from "@/domain/model/api";
import {
  formatModelCapability,
  formatModelProtocol,
  normalizeModelCapabilities,
} from "@/domain/model/constants/modelPageOptions";
import {
  deletePlatformModelRecord,
  duplicatePlatformModelRecord,
  listPlatformModelsMock,
  setPlatformDefaultRecord,
  setPlatformModelEnabled,
  verifyPlatformModelRecord,
} from "@/domain/model/mock";
import type { ModelRecord } from "@/domain/model/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

interface RowMenuState {
  left: number;
  modelCode: string;
  top: number;
  triggerElement: HTMLButtonElement;
}

export function PlatformModelsPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(["common", "pages"]);
  const toast = useToast();
  const dataSource = getServiceDataSource("model");
  const [records, setRecords] = useState<ModelRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const rowMenuRef = useRef<HTMLDivElement | null>(null);

  const openMenuRow = openRowMenu
    ? records.find((record) => record.modelCode === openRowMenu.modelCode) ?? null
    : null;
  const pagination = useTablePagination(records);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:modelRegistry.${key}`, { defaultValue, ...options })
  );

  async function refreshPlatformModels() {
    setLoading(true);
    try {
      const nextRecords = dataSource === "api"
        ? await fetchPlatformModels()
        : listPlatformModelsMock();
      setRecords(nextRecords);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Platform models could not be loaded"));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    let active = true;

    async function loadPlatformModels() {
      setLoading(true);
      try {
        const nextRecords = dataSource === "api"
          ? await fetchPlatformModels()
          : listPlatformModelsMock();
        if (active) setRecords(nextRecords);
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : pageT("errors.load", "Platform models could not be loaded"));
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadPlatformModels();

    return () => {
      active = false;
    };
  }, [dataSource, toast]);

  useDismissibleLayer({
    onDismiss: () => setOpenRowMenu(null),
    open: openRowMenu !== null,
    popupRef: rowMenuRef,
    triggerElement: openRowMenu?.triggerElement,
  });

  function toggleRowMenu(row: ModelRecord, trigger: HTMLButtonElement) {
    if (rowBusyCode === row.modelCode) return;
    if (openRowMenu?.modelCode === row.modelCode) {
      setOpenRowMenu(null);
      return;
    }

    const menuWidth = 176;
    const estimatedMenuHeight = 236;
    const rect = trigger.getBoundingClientRect();
    const left = Math.min(
      Math.max(12, rect.right - menuWidth),
      window.innerWidth - menuWidth - 12,
    );
    const top = Math.min(
      Math.max(12, rect.bottom + 8),
      window.innerHeight - estimatedMenuHeight - 12,
    );

    setOpenRowMenu({
      modelCode: row.modelCode,
      left,
      top,
      triggerElement: trigger,
    });
  }

  async function handleToggleEnabled(row: ModelRecord) {
    if (rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);

    try {
      const updated = dataSource === "api"
        ? (row.enabled
          ? await disablePlatformModel(row.modelCode)
          : await enablePlatformModel(row.modelCode))
        : setPlatformModelEnabled(row.modelCode, !row.enabled);
      toast.success(pageT(
        "messages.toggleStatus",
        "{{name}} {{status}}",
        {
          name: updated.displayName,
          status: updated.enabled ? pageT("status.enabledLower", "enabled") : pageT("status.disabledLower", "disabled"),
        },
      ));
      await refreshPlatformModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.updateStatus", "Status update failed"));
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleDelete(row: ModelRecord) {
    if (rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);

    try {
      if (dataSource === "api") {
        await deletePlatformModel(row.modelCode);
      } else {
        deletePlatformModelRecord(row.modelCode);
      }
      toast.success(pageT("messages.deleted", "{{name}} deleted", { name: row.displayName }));
      await refreshPlatformModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.delete", "Platform model could not be deleted"));
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleDuplicate(row: ModelRecord) {
    if (rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);

    try {
      const duplicated = dataSource === "api"
        ? await duplicatePlatformModel(row.modelCode)
        : duplicatePlatformModelRecord(row.modelCode);
      toast.success(pageT("messages.created", "{{name}} created", { name: duplicated.displayName }));
      navigate(`/settings/models/detail?modelCode=${encodeURIComponent(duplicated.modelCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.duplicate", "Platform model could not be duplicated"));
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleVerify(row: ModelRecord) {
    if (rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);
    const verifyingToastId = toast.info(pageT("messages.verifying", "Verifying connectivity for {{name}}...", { name: row.displayName }), {
      title: pageT("actions.verify", "Verify"),
      durationMs: 0,
    });

    try {
      const verified = dataSource === "api"
        ? await verifyPlatformModel(row.modelCode)
        : verifyPlatformModelRecord(row.modelCode);
      toast.success(pageT("messages.verified", "{{name}} verified", { name: verified.displayName }));
      await refreshPlatformModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.verify", "Verify failed"));
    } finally {
      toast.dismiss(verifyingToastId);
      setRowBusyCode("");
    }
  }

  async function handleSetDefault(row: ModelRecord) {
    if (rowBusyCode === row.modelCode || row.isDefault || !row.enabled) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);

    try {
      if (dataSource === "api") {
        await setPlatformDefaultModel(row.modelCode);
      } else {
        setPlatformDefaultRecord(row.modelCode);
      }
      toast.success(pageT("messages.default", "{{name}} set as default", { name: row.displayName }));
      await refreshPlatformModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.default", "Default model could not be updated"));
    } finally {
      setRowBusyCode("");
    }
  }

  return (
    <>
      <PageHeader
        title={pageT("title", "Platform Models")}
        actions={(
          <>
            <Button variant="secondary" disabled={loading} onClick={() => void refreshPlatformModels()}>
              <RefreshCcw size={16} aria-hidden="true" />
              {loading ? pageT("actions.refreshing", "Refreshing...") : t("common:actions.refresh")}
            </Button>
            <Button onClick={() => navigate("/settings/models/new")}>
              <Plus size={16} aria-hidden="true" />
              {pageT("actions.new", "New")}
            </Button>
          </>
        )}
      />

      <DataTable<ModelRecord>
        rows={pagination.visibleItems}
        pagination={<TablePagination itemLabel={pageT("pagination.itemLabel", "models")} {...pagination.paginationProps} />}
        rowKey={(row) => row.modelCode}
        onRowClick={(row) => navigate(`/settings/models/detail?modelCode=${encodeURIComponent(row.modelCode)}`)}
        columns={[
          {
            id: "displayName",
            header: pageT("columns.displayName", "Display Name"),
            render: (row) => (
              <div>
                <p className="font-medium text-[var(--color-on-surface)]">{row.displayName}</p>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">{row.modelName}</p>
              </div>
            ),
          },
          {
            id: "vendor",
            header: pageT("columns.vendor", "Vendor"),
            render: (row) => row.vendor,
          },
          {
            id: "protocol",
            header: pageT("columns.protocol", "Protocol"),
              render: (row) => pageT(`protocol.${row.protocol === "OPENAI_COMPATIBLE" ? "openaiCompatible.chat" : row.protocol === "OPENAI_COMPATIBLE_EMBEDDING" ? "openaiCompatible.embedding" : row.protocol === "RESPONSES" ? "responses" : "rerank"}`, formatModelProtocol(row.protocol, row.modelType)),
          },
          {
            id: "type",
            header: pageT("columns.type", "Type"),
              render: (row) => normalizeModelCapabilities(row.modelCapabilities, row.modelType).map((capability) => pageT(`capabilities.${capability}`, formatModelCapability(capability))).join(", "),
          },
          {
            id: "status",
            header: pageT("columns.status", "Status"),
            render: (row) => (
              <div className="flex flex-wrap gap-2">
                <Badge tone={row.enabled ? "success" : "info"}>{row.enabled ? pageT("status.enabled", "Enabled") : pageT("status.disabled", "Disabled")}</Badge>
                {row.isDefault ? <Badge tone="info">{pageT("status.default", "Default")}</Badge> : null}
              </div>
            ),
          },
          {
            align: "right",
            header: pageT("columns.actions", "Actions"),
            headerClassName: "w-24",
            id: "actions",
            cellClassName: "w-24",
            render: (row) => {
              const isMenuOpen = openRowMenu?.modelCode === row.modelCode;
              const isBusy = rowBusyCode === row.modelCode;

              return (
                <div className="flex justify-end">
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label={pageT("aria.openActions", "Open actions for {{name}}", { name: row.displayName })}
                    aria-expanded={isMenuOpen}
                    data-platform-model-menu-trigger={row.modelCode}
                    disabled={isBusy}
                    onClick={(event) => {
                      event.stopPropagation();
                      toggleRowMenu(row, event.currentTarget);
                    }}
                  >
                    <MoreHorizontal size={16} aria-hidden="true" />
                  </Button>
                </div>
              );
            },
          },
        ]}
        emptyState={pageT("empty.platform", "No platform models yet.")}
      />
      {openRowMenu && openMenuRow
        ? createPortal(
          <div
            ref={rowMenuRef}
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
            className="fixed z-[80] w-44 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            aria-label={pageT("aria.menu", "Actions for {{name}}", { name: openMenuRow.displayName })}
          >
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyCode === openMenuRow.modelCode}
              onClick={() => {
                void handleToggleEnabled(openMenuRow);
              }}
            >
              {openMenuRow.enabled ? pageT("actions.disable", "Disable") : pageT("actions.enable", "Enable")}
            </button>
            {openMenuRow.enabled && !openMenuRow.isDefault ? (
              <button
                type="button"
                role="menuitem"
                className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
                disabled={rowBusyCode === openMenuRow.modelCode}
                onClick={() => {
                  void handleSetDefault(openMenuRow);
                }}
              >
                {pageT("actions.setDefault", "Set Default")}
              </button>
            ) : null}
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyCode === openMenuRow.modelCode}
              onClick={() => {
                void handleVerify(openMenuRow);
              }}
            >
              {pageT("actions.verify", "Verify")}
            </button>
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyCode === openMenuRow.modelCode}
              onClick={() => {
                void handleDuplicate(openMenuRow);
              }}
            >
              {pageT("actions.copy", "Copy")}
            </button>
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              onClick={() => {
                setOpenRowMenu(null);
                navigate(`/settings/models/detail?modelCode=${encodeURIComponent(openMenuRow.modelCode)}&mode=edit`);
              }}
            >
              {pageT("actions.edit", "Edit")}
            </button>
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)] disabled:cursor-not-allowed disabled:opacity-60"
              disabled={rowBusyCode === openMenuRow.modelCode}
              onClick={() => {
                void handleDelete(openMenuRow);
              }}
            >
              {pageT("actions.delete", "Delete")}
            </button>
          </div>,
          document.body,
        )
        : null}
    </>
  );
}
