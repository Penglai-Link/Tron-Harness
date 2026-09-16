import { useCallback, useEffect, useMemo, useState } from "react";
import { ArrowRight, RefreshCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { DataTable, type DataTableColumn } from "@/components/data-table";
import { InfoTooltip } from "@/components/info-tooltip";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { useToast } from "@/components/ui/toast";
import { PageHeader, useLocaleFormatters } from "@/shared";
import {
  fetchModelProviderConnections,
  type ModelProviderConnection,
} from "@/domain/model/api";
import { ModelProviderStatusBadge } from "@/domain/model/components/ModelProviderStatusBadge";

function providerDetailPath(providerCode: string) {
  return `/settings/model-providers/detail?providerCode=${encodeURIComponent(providerCode)}`;
}

export function ModelProvidersPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation("pages");
  const { formatDateTime } = useLocaleFormatters();
  const [records, setRecords] = useState<ModelProviderConnection[]>([]);
  const [loading, setLoading] = useState(true);

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
    void load();
  }, [load]);

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
            <p
              className="mt-1 truncate font-mono text-[12px] text-[var(--color-on-surface-variant)]"
              title={provider.providerCode}
            >
              {provider.providerCode}
            </p>
          </div>
        ),
      },
      {
        id: "status",
        header: t("modelProviders.columns.connectionStatus"),
        render: (provider) => (
          <ModelProviderStatusBadge status={provider.status} />
        ),
      },
      {
        id: "catalogModelCount",
        header: t("modelProviders.columns.catalogModels"),
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
            : t("modelProviders.values.neverSynced"),
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
          <div className="flex justify-end">
            <Button
              size="sm"
              variant="ghost"
              onClick={(event) => {
                event.stopPropagation();
                navigate(providerDetailPath(provider.providerCode));
              }}
            >
              {t("modelProviders.actions.manage")}
              <ArrowRight size={14} aria-hidden="true" />
            </Button>
          </div>
        ),
      },
    ],
    [formatDateTime, navigate, t],
  );

  return (
    <>
      <PageHeader
        title={t("modelProviders.titles.providers")}
        actions={
          <Button
            disabled={loading}
            variant="secondary"
            onClick={() => void load()}
          >
            {loading ? (
              <Spinner size={16} aria-hidden="true" />
            ) : (
              <RefreshCcw size={16} aria-hidden="true" />
            )}
            {t("modelProviders.actions.refresh")}
          </Button>
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
        onRowClick={(provider) =>
          navigate(providerDetailPath(provider.providerCode))
        }
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
        tableClassName="min-w-[860px]"
        title={t("modelProviders.columns.provider")}
        toolbar={
          <InfoTooltip ariaLabel={t("modelProviders.aria.providerListHelp")}>
            {t("modelProviders.help.providerList")}
          </InfoTooltip>
        }
      />
    </>
  );
}
