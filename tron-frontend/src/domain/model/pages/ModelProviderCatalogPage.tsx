import { useCallback, useEffect, useMemo, useState } from "react";

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
  syncModelProvider,
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
type CatalogQualityFilter = "ALL" | "UNKNOWN" | "MULTIMEDIA" | ProviderModelQuality;

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

  const load = useCallback(async (sync = false) => {
    setLoading(true);
    try {
      if (sync) {
        await syncModelProvider(personAiProviderCode);
      }
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
        || (qualityFilter === "MULTIMEDIA" ? model.catalogModelType !== "Chat"
          : qualityFilter === "UNKNOWN" ? model.catalogModelType === "Chat" && model.qualityType === null
          : model.catalogModelType === "Chat" && model.qualityType === qualityFilter);
      return matchesProtocol && matchesQuality;
    }),
    [protocolFilter, qualityFilter, records],
  );

  const columns = useMemo<DataTableColumn<ProviderCatalogModel>[]>(
    () => [
      {
        id: "vendor",
        header: (
          <ColumnHeader
            help={t("modelProviders.help.upstreamVendor")}
            label={t("modelProviders.columns.upstreamVendor")}
          />
        ),
        render: (model) => <div className="min-w-0">
          <p className="truncate" title={model.upstreamVendorName ?? undefined}>{model.upstreamVendorName ?? t("modelProviders.values.unknown")}</p>
          <p className="mt-1 truncate text-xs text-[var(--color-on-surface-variant)]">{model.upstreamVendorCode || t("modelProviders.values.dash")}</p>
        </div>,
      },
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
        id: "tags",
        header: t("modelProviders.columns.tags"),
        render: (model) => model.tags?.length ? <div className="flex flex-wrap gap-1">{model.tags.map(tag => <div key={tag}><Badge variant="outline">{t(`modelProviders.modelTags.${tag}`, { defaultValue: tag })}</Badge>{(tag === "text_embedding" || tag === "multimodal_embedding") && <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">{model.dimension == null ? t("modelProviders.dimensionUnknown") : t("modelProviders.dimensionValue", { count: model.dimension })}</p>}</div>)}</div> : t("modelProviders.values.dash"),
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
        render: (model) => model.catalogModelType !== "Chat" ? <Badge variant="outline">{t("modelProviders.quality.Multimedia")}</Badge> : model.qualityType ? (
          <Badge variant="outline">
            {t(`modelProviders.quality.${model.qualityType}`)}
          </Badge>
        ) : t("modelProviders.values.dash"),
      },
      {
        id: "coefficients",
        header: t("modelProviders.columns.coefficients"),
        render: (model) => <div className="space-y-1 text-xs">{([
          ["inputCoefficient", model.inputCoefficient],
          ["cacheHitCoefficient", model.cacheHitCoefficient],
          ["outputCoefficient", model.outputCoefficient],
        ] as const).map(([key, value]) => <div key={key} className="whitespace-nowrap"><span>{t(`modelProviders.columns.${key}`)}: </span>{value == null ? t("modelProviders.values.dash") : formatCoefficient(value)}</div>)}</div>,
      },
      {
        id: "status",
        header: t("modelProviders.columns.status"),
        render: (model) => <Badge variant={model.status === "Available" ? "success" : "secondary"} className={model.status === "Available" ? "font-semibold" : "opacity-55 font-normal"}>{model.status === "Unavailable" ? t("modelProviders.catalogDisabled") : t(`modelProviders.status.${model.status}`)}</Badge>,
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

              {t("modelProviders.actions.backToConnection")}
            </Button>
            <Button
              disabled={loading}
              variant="secondary"
              onClick={() => void load(true)}
            >

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
        rowKey={(model) => JSON.stringify([
          model.catalogModelType, model.modelId ?? model.modelCode,
          model.upstreamVendorCode, model.qualityType,
        ])}
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
              <SelectItem value="MULTIMEDIA">{t("modelProviders.quality.Multimedia")}</SelectItem>
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
