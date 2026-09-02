import {
  useEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router";
import {
  Edit3,
  MoreHorizontal,
  Plus,
  Power,
  PowerOff,
  RefreshCcw,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  deletePlatformMcpServer,
  disablePlatformMcpServer,
  enablePlatformMcpServer,
  fetchPlatformMcpGateway,
  fetchPlatformMcpServers,
  updatePlatformMcpGateway,
  verifyPlatformMcpServer,
} from "@/domain/mcp/api";
import {
  deletePlatformMcpServerMock,
  disablePlatformMcpServerMock,
  enablePlatformMcpServerMock,
  getPlatformMcpGatewayMock,
  listPlatformMcpServersMock,
  updatePlatformMcpGatewayMock,
  verifyPlatformMcpServerMock,
} from "@/domain/mcp/mock";
import type { PlatformMcpGatewayConfig, PlatformMcpServerRecord } from "@/domain/mcp/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Badge,
  Button,
  DataTable,
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  TablePagination,
  ToggleSwitch,
  useLocaleFormatters,
  useTablePagination,
  useToast,
} from "@/shared";

import {
  formatPlatformMcpEndpoint,
  formatPlatformMcpTransportLabel,
  platformMcpServerEnabledTone,
  platformMcpServerStateTone,
} from "./mcpServerPageHelpers";
import { SelectItem } from "@/components/ui/select";

type StatusFilter = "all" | "enabled" | "disabled" | "healthy" | "unhealthy" | "never_checked";

interface RowMenuState {
  left: number;
  serverCode: string;
  top: number;
  triggerElement: HTMLButtonElement;
}

function healthLabel(status: PlatformMcpServerRecord["lastHealthStatus"]) {
  return status.replaceAll("_", " ");
}

export function PlatformMcpServersPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const dataSource = getServiceDataSource("mcp");
  const [gateway, setGateway] = useState<PlatformMcpGatewayConfig | null>(null);
  const [records, setRecords] = useState<PlatformMcpServerRecord[]>([]);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [loading, setLoading] = useState(false);
  const [savingModules, setSavingModules] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const rowMenuRef = useRef<HTMLDivElement | null>(null);

  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:settingsMcpServers.${key}`, { defaultValue, ...options })
  );

  const pagination = useTablePagination(records);
  const openMenuRow = openRowMenu
    ? records.find((record) => record.serverCode === openRowMenu.serverCode) ?? null
    : null;

  async function refreshPage() {
    setLoading(true);
    try {
      const [nextGateway, nextRecords] = dataSource === "api"
        ? await Promise.all([
          fetchPlatformMcpGateway(),
          fetchPlatformMcpServers({
            search,
            status: statusFilter === "all" ? undefined : statusFilter,
          }),
        ])
        : await Promise.all([
          Promise.resolve(getPlatformMcpGatewayMock()),
          Promise.resolve(listPlatformMcpServersMock({
            search,
            status: statusFilter === "all" ? undefined : statusFilter,
          })),
        ]);
      setGateway(nextGateway);
      setRecords(nextRecords);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Platform MCP settings could not be loaded"));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refreshPage();
  }, [dataSource, search, statusFilter]);

  useEffect(() => {
    if (!openRowMenu) return undefined;

    const handleDocumentClick = (event: MouseEvent) => {
      const target = event.target as Node;
      if (rowMenuRef.current?.contains(target)) return;
      if (openRowMenu.triggerElement.contains(target)) return;
      setOpenRowMenu(null);
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpenRowMenu(null);
    };

    document.addEventListener("click", handleDocumentClick);
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("click", handleDocumentClick);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [openRowMenu]);

  function toggleRowMenu(row: PlatformMcpServerRecord, trigger: HTMLButtonElement) {
    if (rowBusyCode === row.serverCode) return;
    if (openRowMenu?.serverCode === row.serverCode) {
      setOpenRowMenu(null);
      return;
    }

    const menuWidth = 176;
    const estimatedMenuHeight = 224;
    const rect = trigger.getBoundingClientRect();
    const left = Math.min(Math.max(12, rect.right - menuWidth), window.innerWidth - menuWidth - 12);
    const top = Math.min(Math.max(12, rect.bottom + 8), window.innerHeight - estimatedMenuHeight - 12);

    setOpenRowMenu({
      left,
      serverCode: row.serverCode,
      top,
      triggerElement: trigger,
    });
  }

  async function handleVariablesEndpointToggle(nextEnabled: boolean) {
    if (!gateway || savingModules) return;
    setSavingModules(true);
    try {
      const updated = dataSource === "api"
        ? await updatePlatformMcpGateway({
          gatewayEnabled: true,
          variablesEndpointEnabled: nextEnabled,
        })
        : updatePlatformMcpGatewayMock({
          gatewayEnabled: true,
          variablesEndpointEnabled: nextEnabled,
        });
      setGateway(updated);
      toast.success(pageT("messages.gatewaySaved", "Module settings saved"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.gateway", "Module settings could not be updated"));
    } finally {
      setSavingModules(false);
    }
  }

  async function handleVerify(row: PlatformMcpServerRecord) {
    if (rowBusyCode === row.serverCode) return;
    setRowBusyCode(row.serverCode);
    setOpenRowMenu(null);
    try {
      const verified = dataSource === "api"
        ? await verifyPlatformMcpServer(row.serverCode)
        : verifyPlatformMcpServerMock(row.serverCode);
      toast.success(pageT("messages.verified", "{{name}} verified", { name: verified.name }));
      await refreshPage();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.verify", "Verify failed"));
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleToggleEnabled(row: PlatformMcpServerRecord) {
    if (rowBusyCode === row.serverCode) return;
    setRowBusyCode(row.serverCode);
    setOpenRowMenu(null);

    try {
      const updated = dataSource === "api"
        ? (row.enabled
          ? await disablePlatformMcpServer(row.serverCode)
          : await enablePlatformMcpServer(row.serverCode))
        : (row.enabled
          ? disablePlatformMcpServerMock(row.serverCode)
          : enablePlatformMcpServerMock(row.serverCode));
      toast.success(pageT(
        "messages.toggleStatus",
        "{{name}} {{status}}",
        {
          name: updated.name,
          status: updated.enabled ? pageT("status.enabledLower", "enabled") : pageT("status.disabledLower", "disabled"),
        },
      ));
      await refreshPage();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.toggle", "Status update failed"));
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleDelete(row: PlatformMcpServerRecord) {
    if (rowBusyCode === row.serverCode) return;
    setRowBusyCode(row.serverCode);
    setOpenRowMenu(null);

    try {
      if (dataSource === "api") {
        await deletePlatformMcpServer(row.serverCode);
      } else {
        deletePlatformMcpServerMock(row.serverCode);
      }
      toast.success(pageT("messages.deleted", "{{name}} deleted", { name: row.name }));
      await refreshPage();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.delete", "Platform MCP server could not be deleted"));
    } finally {
      setRowBusyCode("");
    }
  }

  return (
    <>
      <PageHeader
        title={pageT("title", "Platform MCP Servers")}
        actions={(
          <>
            <Button variant="secondary" disabled={loading} aria-busy={loading} onClick={() => void refreshPage()}>
              <RefreshCcw size={16} aria-hidden="true" />
              {loading ? pageT("actions.refreshing", "Refreshing...") : pageT("actions.refresh", "Refresh")}
            </Button>
            <Button onClick={() => navigate("/settings/mcp-servers/new")}>
              <Plus size={16} aria-hidden="true" />
              {pageT("actions.new", "New")}
            </Button>
          </>
        )}
      />

      <div className="space-y-5">
        <SurfaceCard className="space-y-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h2 className="text-lg font-semibold">{pageT("gateway.title", "Internal modules")}</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                {pageT("gateway.description", "Manage platform-internal MCP modules exposed through the gateway.")}
              </p>
            </div>
            <p className="text-xs text-[var(--color-on-surface-variant)]">
              {gateway ? pageT("gateway.updatedAt", "Updated {{value}}", {
                value: formatDateTime(gateway.updatedAt, { dateStyle: "medium", timeStyle: "short" }),
              }) : ""}
            </p>
          </div>

          {gateway ? (
            <div className="grid gap-4">
              <ModuleSwitchRow
                checked={gateway.variablesEndpointEnabled}
                description={pageT("gateway.variablesEnabledHint", "Controls whether `/mcp/variables` is exposed through the gateway.")}
                disabled={savingModules}
                label={pageT("gateway.variablesEndpointEnabled", "Variables endpoint")}
                onToggle={(checked) => void handleVariablesEndpointToggle(checked)}
              />
            </div>
          ) : null}
        </SurfaceCard>

        <DataTable<PlatformMcpServerRecord>
          title={pageT("table.title", "Platform registry")}
          rows={pagination.visibleItems}
          rowKey={(row) => row.serverCode}
          onRowClick={(row) => navigate(`/settings/mcp-servers/detail?serverCode=${encodeURIComponent(row.serverCode)}`)}
          pagination={<TablePagination itemLabel={pageT("table.itemLabel", "servers")} {...pagination.paginationProps} />}
          toolbar={(
            <>
              <SearchField
                ariaLabel={pageT("table.searchAria", "Search MCP servers")}
                className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
                placeholder={pageT("table.searchPlaceholder", "Search MCP servers...")}
                value={search}
                onChange={setSearch}
              />
              <SelectInput
                aria-label={pageT("table.statusAria", "Filter MCP server status")}
                className="w-full sm:w-48"
                value={statusFilter}
                onValueChange={(event) => setStatusFilter(event as StatusFilter)}
              >
                <SelectItem value="all">{pageT("table.status.all", "All statuses")}</SelectItem>
                <SelectItem value="enabled">{pageT("table.status.enabled", "Enabled")}</SelectItem>
                <SelectItem value="disabled">{pageT("table.status.disabled", "Disabled")}</SelectItem>
                <SelectItem value="healthy">{pageT("table.status.healthy", "Healthy")}</SelectItem>
                <SelectItem value="unhealthy">{pageT("table.status.unhealthy", "Unhealthy")}</SelectItem>
                <SelectItem value="never_checked">{pageT("table.status.neverChecked", "Never checked")}</SelectItem>
              </SelectInput>
            </>
          )}
          emptyState={pageT("table.empty", "No MCP servers match the current filters.")}
          columns={[
            {
              id: "server",
              header: pageT("table.columns.server", "Server"),
              width: "24%",
              render: (row) => (
                <div>
                  <p className="font-medium text-[var(--color-on-surface)]">{row.name}</p>
                  <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{row.serverCode}</p>
                  {row.description ? (
                    <p className="mt-1 line-clamp-2 text-[13px] text-[var(--color-on-surface-variant)]">{row.description}</p>
                  ) : null}
                </div>
              ),
            },
            {
              id: "transport",
              header: pageT("table.columns.transport", "Transport"),
              width: "14%",
              render: (row) => (
                <Badge tone="info">
                  {formatPlatformMcpTransportLabel(row.transportKind)}
                </Badge>
              ),
            },
            {
              id: "endpoint",
              header: pageT("table.columns.endpoint", "Endpoint"),
              width: "24%",
              render: (row) => (
                <span className="block max-w-[28rem] truncate font-mono text-[13px] text-[var(--color-on-surface-variant)]">
                  {formatPlatformMcpEndpoint(row)}
                </span>
              ),
            },
            {
              id: "health",
              header: pageT("table.columns.health", "Health"),
              width: "16%",
              render: (row) => (
                <div>
                  <Badge tone={platformMcpServerStateTone(row.lastHealthStatus)}>
                    {healthLabel(row.lastHealthStatus)}
                  </Badge>
                </div>
              ),
            },
            {
              id: "state",
              header: pageT("table.columns.state", "State"),
              width: "10%",
              render: (row) => (
                <Badge tone={platformMcpServerEnabledTone(row.enabled)}>
                  {row.enabled ? pageT("status.enabled", "Enabled") : pageT("status.disabled", "Disabled")}
                </Badge>
              ),
            },
            {
              align: "right",
              header: pageT("table.columns.actions", "Actions"),
              headerClassName: "w-24",
              id: "actions",
              width: "12%",
              render: (row) => {
                const isMenuOpen = openRowMenu?.serverCode === row.serverCode;
                const isBusy = rowBusyCode === row.serverCode;

                return (
                  <div className="flex justify-end">
                    <Button
                      size="icon"
                      variant="ghost"
                      aria-label={pageT("table.actions.open", "Open actions for {{name}}", { name: row.name })}
                      aria-expanded={isMenuOpen}
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
        />
      </div>

      {openRowMenu && openMenuRow
        ? createPortal(
          <div
            ref={rowMenuRef}
            className="fixed z-50 w-44 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 shadow-[0_18px_36px_rgba(15,23,42,0.16)]"
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
          >
            <button
              className="flex w-full items-center gap-[12px] rounded-lg px-3 py-2 text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
              type="button"
              onClick={() => {
                setOpenRowMenu(null);
                navigate(`/settings/mcp-servers/edit?serverCode=${encodeURIComponent(openMenuRow.serverCode)}`);
              }}
            >
              <Edit3 size={16} aria-hidden="true" />
              {pageT("table.actions.edit", "Edit")}
            </button>
            <button
              className="flex w-full items-center gap-[12px] rounded-lg px-3 py-2 text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
              type="button"
              onClick={() => void handleVerify(openMenuRow)}
            >
              <ShieldCheck size={16} aria-hidden="true" />
              {pageT("table.actions.verify", "Verify")}
            </button>
            <button
              className="flex w-full items-center gap-[12px] rounded-lg px-3 py-2 text-sm text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
              type="button"
              onClick={() => void handleToggleEnabled(openMenuRow)}
            >
              {openMenuRow.enabled ? <PowerOff size={16} aria-hidden="true" /> : <Power size={16} aria-hidden="true" />}
              {openMenuRow.enabled ? pageT("table.actions.disable", "Disable") : pageT("table.actions.enable", "Enable")}
            </button>
            <button
              className="flex w-full items-center gap-[12px] rounded-lg px-3 py-2 text-sm text-[var(--color-error)] hover:bg-[var(--color-error-container)]"
              type="button"
              onClick={() => void handleDelete(openMenuRow)}
            >
              <Trash2 size={16} aria-hidden="true" />
              {pageT("table.actions.delete", "Delete")}
            </button>
          </div>,
          document.body,
        )
        : null}
    </>
  );
}

function ModuleSwitchRow({
  checked,
  description,
  disabled,
  label,
  onToggle,
}: {
  checked: boolean;
  description: string;
  disabled: boolean;
  label: string;
  onToggle: (checked: boolean) => void;
}) {
  return (
    <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
      <div className="flex items-center justify-between gap-3">
        <div className="space-y-1">
          <p className="text-sm font-medium text-[var(--color-on-surface)]">{label}</p>
          <p className="text-[13px] text-[var(--color-on-surface-variant)]">{description}</p>
        </div>
        <ToggleSwitch
          checked={checked}
          disabled={disabled}
          ariaLabel={label}
          onCheckedChange={onToggle}
        />
      </div>
    </div>
  );
}
