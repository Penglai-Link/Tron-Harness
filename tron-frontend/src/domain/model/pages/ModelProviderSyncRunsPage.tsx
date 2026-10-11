import { useCallback, useEffect, useMemo, useState } from "react";

import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { DataTable, type DataTableColumn } from "@/components/data-table";
import { InfoTooltip } from "@/components/info-tooltip";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { useToast } from "@/components/ui/toast";
import { PageHeader, useLocaleFormatters } from "@/shared";
import {
  fetchProviderSyncRuns,
  type ProviderSyncRun,
} from "@/domain/model/api";
import { ProviderSyncStatusBadge } from "@/domain/model/components/ModelProviderStatusBadge";

const personAiProviderCode = "PersonAi" as const;

export function ModelProviderSyncRunsPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation("pages");
  const { formatDateTime } = useLocaleFormatters();
  const [records, setRecords] = useState<ProviderSyncRun[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setRecords(await fetchProviderSyncRuns(personAiProviderCode));
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : t("modelProviders.messages.loadSyncRunsFailed"),
      );
    } finally {
      setLoading(false);
    }
  }, [t, toast]);

  useEffect(() => {
    void load();
  }, [load]);

  const columns = useMemo<DataTableColumn<ProviderSyncRun>[]>(
    () => [
      {
        id: "startedAt",
        header: t("modelProviders.columns.startedAt"),
        width: "24%",
        render: (run) =>
          formatDateTime(run.startedAt, {
            dateStyle: "medium",
            timeStyle: "short",
          }),
      },
      {
        id: "status",
        header: t("modelProviders.columns.status"),
        render: (run) => <ProviderSyncStatusBadge status={run.status} />,
      },
      {
        id: "modelCount",
        header: t("modelProviders.columns.modelCount"),
        render: (run) => run.modelCount ?? t("modelProviders.values.dash"),
      },
      {
        id: "completedAt",
        header: t("modelProviders.columns.completedAt"),
        render: (run) =>
          run.completedAt
            ? formatDateTime(run.completedAt, {
                dateStyle: "medium",
                timeStyle: "short",
              })
            : t("modelProviders.values.dash"),
      },
      {
        id: "summary",
        header: t("modelProviders.columns.result"),
        width: "34%",
        render: (run) => {
          const value =
            run.status === "Succeeded"
              ? t("modelProviders.summary.catalogUpdated")
              : run.status === "Failed"
                ? (run.errorSummary ?? t("modelProviders.summary.unknownError"))
                : t("modelProviders.summary.syncingCatalog");
          return (
            <span
              className="block truncate text-[13px] text-[var(--color-on-surface-variant)]"
              title={value}
            >
              {value}
            </span>
          );
        },
      },
    ],
    [formatDateTime, t],
  );

  return (
    <>
      <PageHeader
        title={t("modelProviders.titles.syncRuns")}
        actions={
          <>
            <Button
              variant="secondary"
              onClick={() =>
                navigate(
                  `/settings/model-providers/detail?providerCode=${personAiProviderCode}`,
                )
              }
            >

              {t("modelProviders.actions.backToConnection")}
            </Button>
            <Button
              disabled={loading}
              variant="secondary"
              onClick={() => void load()}
            >

              {t("modelProviders.actions.refresh")}
            </Button>
          </>
        }
      />

      <DataTable<ProviderSyncRun>
        columns={columns}
        emptyState={
          loading ? (
            <span className="inline-flex items-center gap-2">
              <Spinner size={16} aria-hidden="true" />
              {t("modelProviders.loading")}
            </span>
          ) : (
            t("modelProviders.empty.syncRuns")
          )
        }
        pagination={
          <span>
            {loading
              ? `${t("modelProviders.loading")}…`
              : t("modelProviders.pagination.syncRuns", {
                  count: records.length,
                })}
          </span>
        }
        rowKey={(run) => run.syncRunCode}
        rows={loading ? [] : records}
        tableClassName="min-w-[820px]"
        title={t("modelProviders.titles.syncRuns")}
        toolbar={
          <InfoTooltip ariaLabel={t("modelProviders.aria.syncRunsHelp")}>
            {t("modelProviders.help.syncRuns")}
          </InfoTooltip>
        }
      />
    </>
  );
}
