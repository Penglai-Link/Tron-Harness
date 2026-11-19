import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { DataTable, type DataTableColumn } from "@/components/data-table";
import { useTabVisibility } from "@/components/editable-tabs";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import { useToast } from "@/components/ui/toast";
import { PageHeader, useLocaleFormatters } from "@/shared";
import {
  fetchModelProviderConnections,
  type ModelProviderConnection,
} from "@/domain/model/api";
import { ModelProviderStatusBadge } from "@/domain/model/components/ModelProviderStatusBadge";
import { ModelProviderRowActions } from "@/domain/model/components/ModelProviderRowActions";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { createModelProviderAuthorization, disconnectModelProvider, syncModelProvider } from "@/domain/model/api";
import { deleteConfiguredProvider, fetchConfiguredProvider, fetchProviderPresets, updateConfiguredProvider, type ProviderPreset } from "@/domain/model/api/configuredProviderApi";

function providerDetailPath(providerCode: string) {
  return `/settings/model-providers/detail?providerCode=${encodeURIComponent(providerCode)}`;
}

export function ModelProvidersPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation("pages");
  const { formatDateTime } = useLocaleFormatters();
  const visible = useTabVisibility();
  const [presets, setPresets] = useState<ProviderPreset[]>([]);
  const [records, setRecords] = useState<ModelProviderConnection[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyCode, setBusyCode] = useState("");
  const actionInFlight = useRef(false);
  const [confirmation, setConfirmation] = useState<{ provider: ModelProviderConnection; kind: "delete" | "disconnect" } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setRecords(await fetchModelProviderConnections());
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : t("modelProviders.messages.loadProvidersFailed"),
      );
    } finally {
      setLoading(false);
    }
  }, [t, toast]);

  useEffect(() => {
    const controller = new AbortController();
    fetchProviderPresets(controller.signal).then(setPresets).catch(() => {});
    return () => controller.abort();
  }, []);

  useEffect(() => {
    if (visible) void load();
  }, [load, visible]);

  useEffect(() => {
    if (!visible) return;
    const refresh = () => { if (!actionInFlight.current) void load(); };
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, [load, visible]);

  const runAction = useCallback(async (provider: ModelProviderConnection, operation: () => Promise<unknown>, successKey: string, errorKey: string, reload = true) => {
    if (actionInFlight.current) return false;
    actionInFlight.current = true;
    setBusyCode(provider.providerCode);
    try {
      await operation();
      toast.success(t(successKey));
      if (reload) await load();
      return true;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t(errorKey));
      return false;
    } finally {
      actionInFlight.current = false;
      setBusyCode("");
    }
  }, [load, t, toast]);

  const toggle = useCallback((provider: ModelProviderConnection) => {
    const enabled = provider.status !== "Connected";
    return runAction(provider, async () => {
      const current = await fetchConfiguredProvider(provider.providerCode);
      await updateConfiguredProvider(provider.providerCode, {
        displayName: current.displayName, protocol: current.protocol, baseUrl: current.baseUrl,
        requestHeaders: current.requestHeaders, defaultReasoningEfforts: current.defaultReasoningEfforts, models: current.models, apiKey: null, enabled,
      });
    }, enabled ? "configuredProviders.messages.enabled" : "configuredProviders.messages.disabled", "configuredProviders.errors.save");
  }, [runAction]);

  const authorize = useCallback((provider: ModelProviderConnection) => runAction(provider, async () => {
    const session = await createModelProviderAuthorization(provider.providerCode);
    window.open(session.loginUrl, "_blank", "noopener,noreferrer");
    navigate(`${providerDetailPath(provider.providerCode)}&authorizationSessionId=${encodeURIComponent(session.sessionId)}`);
  }, "modelProviders.messages.authorizationPageOpened", "modelProviders.messages.openAuthorizationFailed", false), [navigate, runAction]);

  async function confirmAction() {
    if (!confirmation) return;
    const { provider, kind } = confirmation;
    const success = await runAction(provider,
      () => kind === "delete" ? deleteConfiguredProvider(provider.providerCode) : disconnectModelProvider(provider.providerCode),
      kind === "delete" ? "configuredProviders.deleted" : "modelProviders.messages.disconnected",
      kind === "delete" ? "configuredProviders.errors.delete" : "modelProviders.messages.disconnectFailed");
    if (success) setConfirmation(null);
  }

  const columns = useMemo<DataTableColumn<ModelProviderConnection>[]>(
    () => [
      {
        id: "provider",
        header: t("modelProviders.columns.provider"),
        width: "28%",
        render: (provider) => (
          <div className="min-w-0">
            <p
              className="truncate font-medium text-[var(--color-on-surface)]"
              title={provider.displayName}
            >
              {provider.displayName}
            </p>
            <p className="truncate text-xs text-[var(--color-on-surface-variant)]">{provider.source !== "Custom" ? "Person-AI" : provider.providerType === "custom" ? t("configuredProviders.custom") : presets.find((preset) => preset.id === provider.providerType)?.name ?? (provider.providerType || t("configuredProviders.providerTypeUnknown"))}</p>
          </div>
        ),
      },
      {
        id: "source",
        header: t("configuredProviders.source"),
        render: (provider) => t(provider.source === "Custom" ? "configuredProviders.custom" : "configuredProviders.plugin"),
      },
      {
        id: "status",
        header: t("modelProviders.columns.connectionStatus"),
        render: (provider) => provider.source === "Custom" ? (
          <Badge variant={provider.status === "Connected" ? "success" : "outline"}>
            {t(provider.status === "Connected" ? "configuredProviders.enabled" : "configuredProviders.disabled")}
          </Badge>
        ) : <ModelProviderStatusBadge status={provider.status} />,
      },
      {
        id: "catalogModelCount",
        header: t("modelProviders.columns.modelCount"),
        render: (provider) => provider.catalogModelCount,
      },
      {
        id: "lastSyncAt",
        header: t("modelProviders.columns.lastSyncAt"),
        render: (provider) =>
          provider.lastSyncAt
            ? formatDateTime(provider.lastSyncAt, {
                dateStyle: "medium",
                timeStyle: "short",
              })
            : t(provider.source === "Custom" ? "modelProviders.values.dash" : "modelProviders.values.neverSynced"),
      },
      {
        id: "authorizationExpiresAt",
        header: t("modelProviders.columns.authorizationExpiresAt"),
        render: (provider) =>
          provider.authorizationExpiresAt
            ? formatDateTime(provider.authorizationExpiresAt, {
                dateStyle: "medium",
                timeStyle: "short",
              })
            : t("modelProviders.values.dash"),
      },
      {
        align: "right",
        cellClassName: "w-24",
        header: t("modelProviders.columns.actions"),
        headerClassName: "w-24",
        id: "actions",
        render: (provider) => (
          <ModelProviderRowActions provider={provider} busy={Boolean(busyCode) || loading}
            onToggle={() => { void toggle(provider); }} onDelete={() => setConfirmation({ provider, kind: "delete" })}
            onAuthorize={() => { void authorize(provider); }}
            onSync={() => { void runAction(provider, () => syncModelProvider(provider.providerCode), "configuredProviders.messages.syncStarted", "modelProviders.messages.syncFailed"); }}
            onDisconnect={() => setConfirmation({ provider, kind: "disconnect" })} />
        ),
      },
    ],
    [authorize, busyCode, formatDateTime, loading, presets, runAction, t, toggle],
  );

  return (
    <>
      <PageHeader
        title={t("modelProviders.titles.providers")}
        actions={
          <>
          <Button disabled={Boolean(busyCode)} onClick={() => navigate("/settings/model-providers/new")}>
            {t("configuredProviders.actions.add")}
          </Button>
          <Button
            disabled={loading || Boolean(busyCode)}
            variant="secondary"
            onClick={() => void load()}
          >

            {t("modelProviders.actions.refresh")}
          </Button>
          </>
        }
      />

      <DataTable<ModelProviderConnection>
        columns={columns}
        emptyState={
          loading ? (
            <span className="inline-flex items-center gap-2">
              <Spinner size={16} aria-hidden="true" />
              {t("modelProviders.loading")}
            </span>
          ) : (
            t("modelProviders.empty.providers")
          )
        }
        onRowClick={(provider) => { if (!actionInFlight.current) navigate(providerDetailPath(provider.providerCode)); }}
        pagination={
          <span>
            {loading
              ? `${t("modelProviders.loading")}…`
              : t("modelProviders.pagination.providers", {
                  count: records.length,
                })}
          </span>
        }
        rowKey={(provider) => provider.providerCode}
        rows={loading ? [] : records}
        tableClassName="min-w-[860px] [&_thead_th]:bg-[var(--color-background)] [&_thead_th]:text-xs [&_thead_th]:font-semibold [&_thead_th]:text-[var(--color-on-surface-variant)] [&_thead_th:first-child]:rounded-l-md [&_thead_th:last-child]:rounded-r-md"
      />
      <ConfirmDialog open={confirmation !== null}
        title={t(confirmation?.kind === "disconnect" ? "modelProviders.disconnectDialog.title" : "configuredProviders.deleteTitle")}
        description={t(confirmation?.kind === "disconnect" ? "configuredProviders.disconnectNamedDescription" : "configuredProviders.deleteNamedDescription", { name: confirmation?.provider.displayName ?? "" })}
        cancelText={t("configuredProviders.actions.cancel")}
        confirmText={t(confirmation?.kind === "disconnect" ? "modelProviders.actions.disconnect" : "configuredProviders.actions.delete")}
        pending={Boolean(busyCode)} pendingText={t("configuredProviders.actions.processing")}
        onCancel={() => setConfirmation(null)} onConfirm={() => { void confirmAction(); }} />
    </>
  );
}
