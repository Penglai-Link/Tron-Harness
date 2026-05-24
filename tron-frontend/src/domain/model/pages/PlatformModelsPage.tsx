import {
  useEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
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
  formatModelCapabilities,
  formatModelProtocol,
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
}

export function PlatformModelsPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("model");
  const [records, setRecords] = useState<ModelRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const rowMenuRef = useRef<HTMLDivElement | null>(null);
  const rowMenuOpenedAtRef = useRef(0);

  const openMenuRow = openRowMenu
    ? records.find((record) => record.modelCode === openRowMenu.modelCode) ?? null
    : null;

  async function refreshPlatformModels() {
    setLoading(true);
    try {
      const nextRecords = dataSource === "api"
        ? await fetchPlatformModels()
        : listPlatformModelsMock();
      setRecords(nextRecords);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform models could not be loaded");
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
        if (active) toast.error(error instanceof Error ? error.message : "Platform models could not be loaded");
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadPlatformModels();

    return () => {
      active = false;
    };
  }, [dataSource, toast]);

  useEffect(() => {
    if (!openRowMenu) return;

    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (rowMenuRef.current?.contains(target)) return;
      const trigger = document.querySelector<HTMLButtonElement>(
        `[data-platform-model-menu-trigger="${openRowMenu.modelCode}"]`,
      );
      if (trigger?.contains(target)) return;
      setOpenRowMenu(null);
    };

    const handleViewportChange = () => {
      if (Date.now() - rowMenuOpenedAtRef.current < 150) return;
      setOpenRowMenu(null);
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpenRowMenu(null);
    };

    document.addEventListener("mousedown", handlePointerDown);
    window.addEventListener("resize", handleViewportChange);
    window.addEventListener("scroll", handleViewportChange, true);
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      window.removeEventListener("resize", handleViewportChange);
      window.removeEventListener("scroll", handleViewportChange, true);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [openRowMenu]);

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
    });
    rowMenuOpenedAtRef.current = Date.now();
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
      toast.success(`${updated.displayName} ${updated.enabled ? "enabled" : "disabled"}`);
      await refreshPlatformModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Status update failed");
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
      toast.success(`${row.displayName} deleted`);
      await refreshPlatformModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform model could not be deleted");
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
      toast.success(`${duplicated.displayName} created`);
      navigate(`/settings/models/detail?modelCode=${encodeURIComponent(duplicated.modelCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform model could not be duplicated");
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleVerify(row: ModelRecord) {
    if (rowBusyCode === row.modelCode) return;
    setRowBusyCode(row.modelCode);
    setOpenRowMenu(null);
    const verifyingToastId = toast.info(`Verifying connectivity for ${row.displayName}...`, {
      title: "Verify",
      durationMs: 0,
    });

    try {
      const verified = dataSource === "api"
        ? await verifyPlatformModel(row.modelCode)
        : verifyPlatformModelRecord(row.modelCode);
      toast.success(`${verified.displayName} verified`);
      await refreshPlatformModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Verify failed");
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
      toast.success(`${row.displayName} set as default`);
      await refreshPlatformModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Default model could not be updated");
    } finally {
      setRowBusyCode("");
    }
  }

  return (
    <>
      <PageHeader
        title="Platform Models"
        actions={(
          <>
            <Button variant="secondary" disabled={loading} onClick={() => void refreshPlatformModels()}>
              <RefreshCcw size={16} aria-hidden="true" />
              {loading ? "Refreshing..." : "Refresh"}
            </Button>
            <Button onClick={() => navigate("/settings/models/new")}>
              <Plus size={16} aria-hidden="true" />
              New
            </Button>
          </>
        )}
      />

      <DataTable<ModelRecord>
        rows={records}
        rowKey={(row) => row.modelCode}
        onRowClick={(row) => navigate(`/settings/models/detail?modelCode=${encodeURIComponent(row.modelCode)}`)}
        columns={[
          {
            id: "displayName",
            header: "Display Name",
            render: (row) => (
              <div>
                <p className="font-medium text-[var(--color-on-surface)]">{row.displayName}</p>
                <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">{row.modelName}</p>
              </div>
            ),
          },
          {
            id: "vendor",
            header: "Vendor",
            render: (row) => row.vendor,
          },
          {
            id: "protocol",
            header: "Protocol",
            render: (row) => formatModelProtocol(row.protocol, row.modelType),
          },
          {
            id: "type",
            header: "Type",
            render: (row) => formatModelCapabilities(row.modelCapabilities, row.modelType),
          },
          {
            id: "status",
            header: "Status",
            render: (row) => (
              <div className="flex flex-wrap gap-2">
                <Badge tone={row.enabled ? "success" : "neutral"}>{row.enabled ? "Enabled" : "Disabled"}</Badge>
                {row.isDefault ? <Badge tone="accent">Default</Badge> : null}
              </div>
            ),
          },
          {
            align: "right",
            header: "Actions",
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
                    aria-label={`Open actions for ${row.displayName}`}
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
        emptyState="No platform models yet."
      />
      {openRowMenu && openMenuRow
        ? createPortal(
          <div
            ref={rowMenuRef}
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
            className="fixed z-[80] w-44 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            aria-label={`Actions for ${openMenuRow.displayName}`}
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
              {openMenuRow.enabled ? "Disable" : "Enable"}
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
                Set Default
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
              Verify
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
              Copy
            </button>
            <button
              type="button"
              role="menuitem"
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              onClick={() => {
                setOpenRowMenu(null);
                navigate(`/settings/models/detail?modelCode=${encodeURIComponent(openMenuRow.modelCode)}`);
              }}
            >
              Edit
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
              Delete
            </button>
          </div>,
          document.body,
        )
        : null}
    </>
  );
}
