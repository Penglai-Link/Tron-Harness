import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import {
  MoreHorizontal,
  Plus,
  RefreshCcw,
} from "lucide-react";

import {
  createPlatformMcpServer,
  deletePlatformMcpServer,
  disablePlatformMcpServer,
  enablePlatformMcpServer,
  fetchPlatformMcpGateway,
  fetchPlatformMcpServers,
  healthCheckPlatformMcpServer,
  updatePlatformMcpGateway,
  updatePlatformMcpServer,
} from "@/domain/mcp/api";
import {
  createPlatformMcpServerMock,
  deletePlatformMcpServerMock,
  getPlatformMcpGatewayMock,
  listPlatformMcpServersMock,
  runPlatformMcpServerHealthCheckMock,
  setPlatformMcpServerEnabledMock,
  updatePlatformMcpGatewayMock,
  updatePlatformMcpServerMock,
} from "@/domain/mcp/mock";
import type {
  McpAuthKind,
  PlatformMcpGatewayConfig,
  PlatformMcpServerPayload,
  PlatformMcpServerRecord,
} from "@/domain/mcp/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Badge,
  Button,
  DataTable,
  Field,
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  ToggleSwitch,
  useLocaleFormatters,
  useTablePagination,
  useToast,
} from "@/shared";

type StatusFilter = "all" | "enabled" | "disabled" | "healthy" | "unhealthy" | "never_checked";
type FormMode = "create" | "edit";

interface RowMenuState {
  left: number;
  serverCode: string;
  top: number;
  triggerElement: HTMLButtonElement;
}

interface ServerFormState {
  authKind: McpAuthKind;
  authPayloadText: string;
  description: string;
  headersText: string;
  name: string;
  serverCode: string;
  url: string;
}

function emptyForm(): ServerFormState {
  return {
    authKind: "bearer_static",
    authPayloadText: '{\n  "tokenRef": ""\n}',
    description: "",
    headersText: "{}",
    name: "",
    serverCode: "",
    url: "",
  };
}

function formFromRecord(record: PlatformMcpServerRecord): ServerFormState {
  return {
    authKind: record.authKind,
    authPayloadText: JSON.stringify(record.authPayload, null, 2),
    description: record.description,
    headersText: JSON.stringify(record.headers, null, 2),
    name: record.name,
    serverCode: record.serverCode,
    url: record.url,
  };
}

function parseJsonMap(value: string, fieldName: string) {
  if (!value.trim()) {
    return {};
  }

  const parsed = JSON.parse(value) as unknown;
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error(`${fieldName} must be a JSON object.`);
  }

  return Object.fromEntries(
    Object.entries(parsed).map(([key, entryValue]) => [key, String(entryValue ?? "")]),
  );
}

function healthTone(status: PlatformMcpServerRecord["lastHealthStatus"]) {
  switch (status) {
    case "HEALTHY":
      return "success" as const;
    case "UNHEALTHY":
      return "warning" as const;
    default:
      return "neutral" as const;
  }
}

export function PlatformMcpServersPage() {
  const { t } = useTranslation(["pages", "shell"]);
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const dataSource = getServiceDataSource("mcp");
  const [gateway, setGateway] = useState<PlatformMcpGatewayConfig | null>(null);
  const [records, setRecords] = useState<PlatformMcpServerRecord[]>([]);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [loading, setLoading] = useState(false);
  const [savingGateway, setSavingGateway] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const [formBusy, setFormBusy] = useState(false);
  const [formMode, setFormMode] = useState<FormMode>("create");
  const [form, setForm] = useState<ServerFormState>(emptyForm());
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

  function beginCreate() {
    setFormMode("create");
    setForm(emptyForm());
  }

  function beginEdit(record: PlatformMcpServerRecord) {
    setFormMode("edit");
    setForm(formFromRecord(record));
    setOpenRowMenu(null);
  }

  function toggleRowMenu(row: PlatformMcpServerRecord, trigger: HTMLButtonElement) {
    if (rowBusyCode === row.serverCode) return;
    if (openRowMenu?.serverCode === row.serverCode) {
      setOpenRowMenu(null);
      return;
    }

    const menuWidth = 176;
    const estimatedMenuHeight = 184;
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

  async function handleGatewayToggle(next: Partial<PlatformMcpGatewayConfig>) {
    if (!gateway || savingGateway) return;
    setSavingGateway(true);
    try {
      const updated = dataSource === "api"
        ? await updatePlatformMcpGateway({
          gatewayEnabled: next.gatewayEnabled ?? gateway.gatewayEnabled,
          variablesEndpointEnabled: next.variablesEndpointEnabled ?? gateway.variablesEndpointEnabled,
        })
        : updatePlatformMcpGatewayMock({
          gatewayEnabled: next.gatewayEnabled ?? gateway.gatewayEnabled,
          variablesEndpointEnabled: next.variablesEndpointEnabled ?? gateway.variablesEndpointEnabled,
        });
      setGateway(updated);
      toast.success(pageT("messages.gatewaySaved", "Gateway settings saved"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.gateway", "Gateway settings could not be updated"));
    } finally {
      setSavingGateway(false);
    }
  }

  async function handleSaveForm() {
    setFormBusy(true);
    try {
      const payload: PlatformMcpServerPayload = {
        authKind: form.authKind,
        authPayload: parseJsonMap(form.authPayloadText, pageT("form.authPayload", "Auth payload")),
        description: form.description.trim(),
        headers: parseJsonMap(form.headersText, pageT("form.headers", "Headers")),
        name: form.name.trim(),
        serverCode: form.serverCode.trim(),
        url: form.url.trim(),
      };

      if (formMode === "create") {
        if (dataSource === "api") {
          await createPlatformMcpServer(payload);
        } else {
          createPlatformMcpServerMock(payload);
        }
        toast.success(pageT("messages.created", "{{name}} created", { name: payload.name }));
        beginCreate();
      } else if (dataSource === "api") {
        await updatePlatformMcpServer(form.serverCode, payload);
        toast.success(pageT("messages.updated", "{{name}} updated", { name: payload.name }));
      } else {
        updatePlatformMcpServerMock(form.serverCode, payload);
        toast.success(pageT("messages.updated", "{{name}} updated", { name: payload.name }));
      }

      await refreshPage();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.save", "MCP server could not be saved"));
    } finally {
      setFormBusy(false);
    }
  }

  async function handleHealthCheck(row: PlatformMcpServerRecord) {
    if (rowBusyCode === row.serverCode) return;
    setRowBusyCode(row.serverCode);
    setOpenRowMenu(null);
    try {
      if (dataSource === "api") {
        await healthCheckPlatformMcpServer(row.serverCode);
      } else {
        runPlatformMcpServerHealthCheckMock(row.serverCode);
      }
      toast.success(pageT("messages.healthChecked", "{{name}} health check completed", { name: row.name }));
      await refreshPage();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.health", "Health check failed"));
    } finally {
      setRowBusyCode("");
    }
  }

  async function handleToggleEnabled(row: PlatformMcpServerRecord) {
    if (rowBusyCode === row.serverCode) return;
    setRowBusyCode(row.serverCode);
    setOpenRowMenu(null);
    try {
      if (dataSource === "api") {
        if (row.enabled) {
          await disablePlatformMcpServer(row.serverCode);
        } else {
          await enablePlatformMcpServer(row.serverCode);
        }
      } else {
        setPlatformMcpServerEnabledMock(row.serverCode, !row.enabled);
      }
      toast.success(pageT(
        row.enabled ? "messages.disabled" : "messages.enabled",
        row.enabled ? "{{name}} disabled" : "{{name}} enabled",
        { name: row.name },
      ));
      await refreshPage();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.toggle", "MCP server state could not be updated"));
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
      if (form.serverCode === row.serverCode) {
        beginCreate();
      }
      await refreshPage();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.delete", "MCP server could not be deleted"));
    } finally {
      setRowBusyCode("");
    }
  }

  const formTitle = formMode === "create"
    ? pageT("form.createTitle", "Create platform MCP server")
    : pageT("form.editTitle", "Edit platform MCP server");

  return (
    <>
      <PageHeader
        title={pageT("title", "Platform MCP Servers")}
        actions={(
          <>
            <Button variant="secondary" disabled={loading} onClick={() => void refreshPage()}>
              <RefreshCcw size={16} aria-hidden="true" />
              {loading ? pageT("actions.refreshing", "Refreshing...") : pageT("actions.refresh", "Refresh")}
            </Button>
            <Button variant="secondary" onClick={beginCreate}>
              <Plus size={16} aria-hidden="true" />
              {pageT("actions.new", "New server")}
            </Button>
          </>
        )}
      />

      <div className="space-y-5">
        <SurfaceCard className="space-y-4 p-5">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div>
              <h2 className="text-base font-semibold text-[var(--color-on-surface)]">
                {pageT("gateway.title", "Gateway")}
              </h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                {pageT("gateway.description", "Manage the platform MCP gateway and the variables transport endpoint.")}
              </p>
            </div>
            {gateway ? (
              <p className="text-xs text-[var(--color-on-surface-variant)]">
                {pageT("gateway.updatedAt", "Updated {{value}}", {
                  value: formatDateTime(gateway.updatedAt, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  }),
                })}
              </p>
            ) : null}
          </div>

          {gateway ? (
            <div className="grid gap-3 md:grid-cols-2">
              <div className="rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-surface-container-low)] p-4">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="text-sm font-medium text-[var(--color-on-surface)]">
                      {pageT("gateway.gatewayEnabled", "Gateway enabled")}
                    </p>
                    <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">
                      {pageT("gateway.gatewayHint", "Controls whether the inbound platform MCP gateway is available.")}
                    </p>
                  </div>
                  <ToggleSwitch
                    checked={gateway.gatewayEnabled}
                    disabled={savingGateway}
                    label={pageT("gateway.gatewayEnabled", "Gateway enabled")}
                    onCheckedChange={(checked) => {
                      void handleGatewayToggle({ gatewayEnabled: checked });
                    }}
                  />
                </div>
              </div>
              <div className="rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-surface-container-low)] p-4">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="text-sm font-medium text-[var(--color-on-surface)]">
                      {pageT("gateway.variablesEndpointEnabled", "Variables endpoint enabled")}
                    </p>
                    <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">
                      {pageT("gateway.variablesHint", "Controls whether `/mcp/variables` is exposed through the gateway.")}
                    </p>
                  </div>
                  <ToggleSwitch
                    checked={gateway.variablesEndpointEnabled}
                    disabled={savingGateway}
                    label={pageT("gateway.variablesEndpointEnabled", "Variables endpoint enabled")}
                    onCheckedChange={(checked) => {
                      void handleGatewayToggle({ variablesEndpointEnabled: checked });
                    }}
                  />
                </div>
              </div>
            </div>
          ) : null}
        </SurfaceCard>

        <DataTable<PlatformMcpServerRecord>
          title={pageT("registry.title", "Platform registry")}
          rows={pagination.visibleItems}
          rowKey={(row) => row.serverCode}
          pagination={<TablePagination itemLabel={pageT("registry.itemLabel", "MCP servers")} {...pagination.paginationProps} />}
          emptyState={pageT("registry.empty", "No MCP servers match the current filters.")}
          toolbar={(
            <>
              <SearchField
                ariaLabel={pageT("registry.searchAria", "Search MCP servers")}
                className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[320px]"
                placeholder={pageT("registry.searchPlaceholder", "Search MCP servers...")}
                value={search}
                onChange={setSearch}
              />
              <SelectInput
                aria-label={pageT("registry.statusAria", "Filter MCP servers by status")}
                className="h-9 min-w-40"
                value={statusFilter}
                onChange={(event) => setStatusFilter(event.target.value as StatusFilter)}
              >
                <option value="all">{pageT("registry.status.all", "All statuses")}</option>
                <option value="enabled">{pageT("registry.status.enabled", "Enabled")}</option>
                <option value="disabled">{pageT("registry.status.disabled", "Disabled")}</option>
                <option value="healthy">{pageT("registry.status.healthy", "Healthy")}</option>
                <option value="unhealthy">{pageT("registry.status.unhealthy", "Unhealthy")}</option>
                <option value="never_checked">{pageT("registry.status.neverChecked", "Never checked")}</option>
              </SelectInput>
            </>
          )}
          columns={[
            {
              id: "server",
              header: pageT("registry.columns.server", "Server"),
              width: "24%",
              render: (row) => (
                <div>
                  <p className="font-medium text-[var(--color-on-surface)]">{row.name}</p>
                  <p className="mt-1 font-mono text-[12px] text-[var(--color-on-surface-variant)]">{row.serverCode}</p>
                </div>
              ),
            },
            {
              id: "description",
              header: pageT("registry.columns.description", "Description"),
              width: "24%",
              render: (row) => (
                <p className="text-[13px] text-[var(--color-on-surface-variant)]">
                  {row.description || pageT("registry.noDescription", "No description available.")}
                </p>
              ),
            },
            {
              id: "endpoint",
              header: pageT("registry.columns.endpoint", "Endpoint"),
              width: "24%",
              render: (row) => (
                <span className="block truncate font-mono text-[12px] text-[var(--color-on-surface-variant)]">
                  {row.url}
                </span>
              ),
            },
            {
              id: "health",
              header: pageT("registry.columns.health", "Health"),
              width: "14%",
              render: (row) => (
                <div className="space-y-1">
                  <Badge tone={healthTone(row.lastHealthStatus)}>
                    {row.lastHealthStatus}
                  </Badge>
                  {row.lastHealthCheckedAt ? (
                    <p className="text-[11px] text-[var(--color-on-surface-variant)]">
                      {formatDateTime(row.lastHealthCheckedAt, {
                        dateStyle: "medium",
                        timeStyle: "short",
                      })}
                    </p>
                  ) : null}
                </div>
              ),
            },
            {
              id: "state",
              header: pageT("registry.columns.state", "State"),
              width: "10%",
              render: (row) => (
                <Badge tone={row.enabled ? "success" : "warning"}>
                  {row.enabled ? pageT("registry.enabled", "Enabled") : pageT("registry.disabled", "Disabled")}
                </Badge>
              ),
            },
            {
              id: "actions",
              header: pageT("registry.columns.actions", "Actions"),
              align: "right",
              width: "8%",
              render: (row) => (
                <div className="flex justify-end">
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label={pageT("registry.openActions", "Open actions for {{name}}", { name: row.name })}
                    aria-expanded={openRowMenu?.serverCode === row.serverCode}
                    disabled={rowBusyCode === row.serverCode}
                    onClick={(event) => {
                      event.stopPropagation();
                      toggleRowMenu(row, event.currentTarget);
                    }}
                  >
                    <MoreHorizontal size={16} aria-hidden="true" />
                  </Button>
                </div>
              ),
            },
          ]}
        />

        <SurfaceCard className="space-y-4 p-5">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{formTitle}</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                {pageT("form.description", "Platform owns MCP connection definitions in v1. Workspaces can only inherit and locally disable them.")}
              </p>
            </div>
            {formMode === "edit" ? (
              <Button variant="secondary" onClick={beginCreate}>
                {pageT("actions.cancelEdit", "Cancel edit")}
              </Button>
            ) : null}
          </div>

          <div className="grid gap-3 lg:grid-cols-2">
            <Field label={pageT("form.serverCode", "Server code")}>
              <TextInput
                disabled={formBusy || formMode === "edit"}
                value={form.serverCode}
                onChange={(event) => setForm((current) => ({ ...current, serverCode: event.target.value }))}
              />
            </Field>
            <Field label={pageT("form.name", "Name")}>
              <TextInput
                disabled={formBusy}
                value={form.name}
                onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
              />
            </Field>
            <Field label={pageT("form.url", "URL")}>
              <TextInput
                disabled={formBusy}
                value={form.url}
                onChange={(event) => setForm((current) => ({ ...current, url: event.target.value }))}
              />
            </Field>
            <Field label={pageT("form.authKind", "Auth kind")}>
              <SelectInput
                disabled={formBusy}
                value={form.authKind}
                onChange={(event) => setForm((current) => ({ ...current, authKind: event.target.value as McpAuthKind }))}
              >
                <option value="bearer_static">{pageT("form.authBearer", "Bearer static")}</option>
                <option value="none">{pageT("form.authNone", "None")}</option>
              </SelectInput>
            </Field>
          </div>

          <Field label={pageT("form.descriptionLabel", "Description")}>
            <TextArea
              className="min-h-24"
              disabled={formBusy}
              value={form.description}
              onChange={(event) => setForm((current) => ({ ...current, description: event.target.value }))}
            />
          </Field>

          <div className="grid gap-3 lg:grid-cols-2">
            <Field
              label={pageT("form.authPayload", "Auth payload")}
              hint={pageT("form.authPayloadHint", "Provide a JSON object such as `{ \"tokenRef\": \"MCP_TOKEN\" }`.")}
            >
              <TextArea
                className="min-h-40 font-mono text-[12px]"
                disabled={formBusy}
                value={form.authPayloadText}
                onChange={(event) => setForm((current) => ({ ...current, authPayloadText: event.target.value }))}
              />
            </Field>
            <Field
              label={pageT("form.headers", "Headers")}
              hint={pageT("form.headersHint", "Provide additional HTTP headers as a JSON object.")}
            >
              <TextArea
                className="min-h-40 font-mono text-[12px]"
                disabled={formBusy}
                value={form.headersText}
                onChange={(event) => setForm((current) => ({ ...current, headersText: event.target.value }))}
              />
            </Field>
          </div>

          <div className="flex flex-wrap justify-end gap-2">
            <Button variant="secondary" disabled={formBusy} onClick={beginCreate}>
              {pageT("actions.resetForm", "Reset")}
            </Button>
            <Button disabled={formBusy} onClick={() => void handleSaveForm()}>
              {formBusy ? pageT("actions.saving", "Saving...") : pageT("actions.save", "Save")}
            </Button>
          </div>
        </SurfaceCard>
      </div>

      {openRowMenu && openMenuRow
        ? createPortal(
          <div
            ref={rowMenuRef}
            style={{ left: openRowMenu.left, top: openRowMenu.top }}
            className="fixed z-[80] w-44 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
            role="menu"
            aria-label={pageT("registry.openActions", "Open actions for {{name}}", { name: openMenuRow.name })}
            onClick={(event) => event.stopPropagation()}
          >
            <button
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              role="menuitem"
              type="button"
              onClick={() => beginEdit(openMenuRow)}
            >
              {pageT("actions.edit", "Edit")}
            </button>
            <button
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              disabled={rowBusyCode === openMenuRow.serverCode}
              role="menuitem"
              type="button"
              onClick={() => {
                void handleHealthCheck(openMenuRow);
              }}
            >
              {pageT("actions.health", "Run health check")}
            </button>
            <button
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              disabled={rowBusyCode === openMenuRow.serverCode}
              role="menuitem"
              type="button"
              onClick={() => {
                void handleToggleEnabled(openMenuRow);
              }}
            >
              {openMenuRow.enabled ? pageT("actions.disable", "Disable") : pageT("actions.enable", "Enable")}
            </button>
            <button
              className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)]"
              disabled={rowBusyCode === openMenuRow.serverCode}
              role="menuitem"
              type="button"
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
