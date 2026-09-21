import { useCallback, useEffect, useMemo, useState } from "react";
import { ArrowLeft, RefreshCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { DataTable, type DataTableColumn } from "@/components/data-table";
import { InfoTooltip } from "@/components/info-tooltip";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { useToast } from "@/components/ui/toast";
import { SelectInput } from "@/components/forms";
import { PageHeader, useLocaleFormatters } from "@/shared";
import {
  fetchProviderCatalog,
  type ProviderCatalogProtocol,
  type ProviderCatalogModel,
} from "@/domain/model/api";
import type { ProviderModelQuality } from "@/domain/model/model";

const personAiProviderCode = "PersonAi" as const;
const catalogProtocols: ProviderCatalogProtocol[] = [
  "CHAT_COMPLETIONS",
  "RESPONSES",
  "EMBEDDINGS",
];
const catalogQualities = ["High", "Middle", "Low"] as const;
type CatalogProtocolFilter = "ALL" | ProviderCatalogProtocol;
type CatalogQualityFilter = "ALL" | "UNKNOWN" | ProviderModelQuality;

function ColumnHeader({ label, help }: { label: string; help: string }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span>{label}</span>
      <InfoTooltip ariaLabel={help}>{help}</InfoTooltip>
    </span>
  );
}

export function ModelProviderCatalogPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation("pages");
  const { formatDateTime, formatNumber } = useLocaleFormatters();
  const [records, setRecords] = useState<ProviderCatalogModel[]>([]);
  const [loading, setLoading] = useState(true);
  const [protocolFilter, setProtocolFilter] = useState<CatalogProtocolFilter>("ALL");
  const [qualityFilter, setQualityFilter] = useState<CatalogQualityFilter>("ALL");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setRecords(await fetchProviderCatalog(personAiProviderCode));
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : t("modelProviders.messages.loadCatalogFailed"),
      );
    } finally {
      setLoading(false);
    }
  }, [t, toast]);

  useEffect(() => {
    void load();
  }, [load]);

  const formatCoefficient = useCallback(
    (value: number) => formatNumber(value, { maximumFractionDigits: 6 }),
    [formatNumber],
  );

  const filteredRecords = useMemo(
    () => records.filter((model) => {
      const matchesProtocol =
        protocolFilter === "ALL" || model.protocols.includes(protocolFilter);
      const matchesQuality =
        qualityFilter === "ALL"
        || (qualityFilter === "UNKNOWN"
          ? model.qualityType === null
          : model.qualityType === qualityFilter);
      return matchesProtocol && matchesQuality;
    }),
    [protocolFilter, qualityFilter, records],
  );

  const columns = useMemo<DataTableColumn<ProviderCatalogModel>[]>(
    () => [
      {
        id: "model",
        header: t("modelProviders.columns.model"),
        width: "34%",
        render: (model) => (
          <div className="min-w-0">
            <p
              className="truncate font-medium text-[var(--color-on-surface)]"
              title={model.modelName}
            >
              {model.modelName}
            </p>
            <p
              className="mt-1 truncate font-mono text-[12px] text-[var(--color-on-surface-variant)]"
              title={model.modelCode}
            >
              {model.modelCode}
            </p>
          </div>
        ),
      },
      {
        id: "catalogModelType",
        header: t("modelProviders.columns.type"),
        render: (model) => (
          <Badge variant="outline">
            {t(`modelProviders.catalogModelTypes.${model.catalogModelType}`)}
          </Badge>
        ),
      },
      {
        id: "protocols",
        header: t("modelProviders.columns.protocol"),
        render: (model) => model.protocols?.map((protocol) => (
          <Badge key={protocol} className="mr-1" variant="outline">
            {t(`modelProviders.protocols.${protocol}`)}
          </Badge>
        )),
      },
      {
        id: "vendor",
        header: (
          <ColumnHeader
            help={t("modelProviders.help.upstreamVendor")}
            label={t("modelProviders.columns.upstreamVendor")}
          />
        ),
        render: (model) =>
          model.upstreamVendorName ??
          model.upstreamVendorCode ??
          t("modelProviders.values.unknown"),
      },
      {
        id: "contextWindow",
        header: (
          <ColumnHeader
            help={t("modelProviders.help.contextWindow")}
            label={t("modelProviders.columns.contextWindow")}
          />
        ),
        render: (model) =>
          model.contextWindowTokens
            ? t("modelProviders.values.contextTokens", {
                count: formatNumber(model.contextWindowTokens),
              })
            : t("modelProviders.values.dash"),
      },
      {
        id: "qualityType",
        header: (
          <ColumnHeader
            help={t("modelProviders.help.quality")}
            label={t("modelProviders.columns.quality")}
          />
        ),
        render: (model) => model.qualityType ? (
          <Badge variant="outline">
            {t(`modelProviders.quality.${model.qualityType}`)}
          </Badge>
        ) : t("modelProviders.values.dash"),
      },
      {
        id: "inputCoefficient",
        align: "right",
        header: (
          <ColumnHeader
            help={t("modelProviders.help.inputCoefficient")}
            label={t("modelProviders.columns.inputCoefficient")}
          />
        ),
        render: (model) => model.inputCoefficient === null
          ? t("modelProviders.values.dash")
          : formatCoefficient(model.inputCoefficient),
      },
      {
        id: "cacheHitCoefficient",
        align: "right",
        header: (
          <ColumnHeader
            help={t("modelProviders.help.cacheHitCoefficient")}
            label={t("modelProviders.columns.cacheHitCoefficient")}
          />
        ),
        render: (model) => model.cacheHitCoefficient === null
          ? t("modelProviders.values.dash")
          : formatCoefficient(model.cacheHitCoefficient),
      },
      {
        id: "outputCoefficient",
        align: "right",
        header: (
          <ColumnHeader
            help={t("modelProviders.help.outputCoefficient")}
            label={t("modelProviders.columns.outputCoefficient")}
          />
        ),
        render: (model) => model.outputCoefficient === null
          ? t("modelProviders.values.dash")
          : formatCoefficient(model.outputCoefficient),
      },
      {
        id: "status",
        header: t("modelProviders.columns.status"),
        render: (model) => (
          <Badge variant={model.status === "Available" ? "success" : "outline"}>
            {t(`modelProviders.status.${model.status}`)}
          </Badge>
        ),
      },
      {
        id: "lastSeenAt",
        header: t("modelProviders.columns.lastSeenAt"),
        render: (model) =>
          model.lastSeenAt
            ? formatDateTime(model.lastSeenAt, {
                dateStyle: "medium",
                timeStyle: "short",
              })
            : t("modelProviders.values.dash"),
      },
    ],
    [formatCoefficient, formatDateTime, formatNumber, t],
  );

  return (
    <>
      <PageHeader
        title={t("modelProviders.titles.catalog")}
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
              <ArrowLeft size={16} aria-hidden="true" />
              {t("modelProviders.actions.backToConnection")}
            </Button>
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
          </>
        }
      />

      <DataTable<ProviderCatalogModel>
        columns={columns}
        emptyState={
          loading ? (
            <span className="inline-flex items-center gap-2">
              <Spinner size={16} aria-hidden="true" />
              {t("modelProviders.loading")}
            </span>
          ) : records.length > 0 && filteredRecords.length === 0 ? (
            t("modelProviders.empty.catalogFiltered")
          ) : (
            t("modelProviders.empty.catalog")
          )
        }
        pagination={
          <span>
            {loading
              ? `${t("modelProviders.loading")}…`
              : t("modelProviders.pagination.catalog", {
                  count: filteredRecords.length,
                })}
          </span>
        }
        rowKey={(model) => `${model.catalogModelType}:${model.modelId ?? model.modelCode}`}
        rows={loading ? [] : filteredRecords}
        tableClassName="min-w-[1240px]"
        title={t("modelProviders.columns.catalogModels")}
        toolbar={
          <div className="flex w-full flex-wrap items-center justify-end gap-3">
            <SelectInput
              aria-label={t("modelProviders.filters.protocol")}
              className="w-[160px]"
              value={protocolFilter}
              onValueChange={(value) => setProtocolFilter((value ?? "ALL") as CatalogProtocolFilter)}
            >
              <SelectItem value="ALL">{t("modelProviders.filters.allProtocols")}</SelectItem>
              {catalogProtocols.map((protocol) => (
                <SelectItem key={protocol} value={protocol}>
                  {t(`modelProviders.protocols.${protocol}`)}
                </SelectItem>
              ))}
            </SelectInput>
            <SelectInput
              aria-label={t("modelProviders.filters.quality")}
              className="w-[140px]"
              value={qualityFilter}
              onValueChange={(value) => setQualityFilter((value ?? "ALL") as CatalogQualityFilter)}
            >
              <SelectItem value="ALL">{t("modelProviders.filters.allQualities")}</SelectItem>
              {catalogQualities.map((quality) => (
                <SelectItem key={quality} value={quality}>
                  {t(`modelProviders.quality.${quality}`)}
                </SelectItem>
              ))}
              <SelectItem value="UNKNOWN">{t("modelProviders.filters.unknownQuality")}</SelectItem>
            </SelectInput>
            <InfoTooltip ariaLabel={t("modelProviders.aria.catalogHelp")}>
              {t("modelProviders.help.catalog")}
            </InfoTooltip>
          </div>
        }
      />
    </>
  );
}
