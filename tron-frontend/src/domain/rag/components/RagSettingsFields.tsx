import { useAuthStore } from "@/store/authStore";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Field, FieldLabel } from "@/components/ui/field";
import { InfoTooltip } from "@/components/info-tooltip";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { fetchModelSelectOptions } from "@/domain/model/api/modelApi";
import { fetchAutomaticRouters } from "@/domain/model/api/automaticRouterApi";
import { fetchProviderCatalog } from "@/domain/model/api/modelProviderApi";
import { autoModelRoutes, autoModelRouteCode } from "@/domain/model/constants/autoModelRoutes";
import type { RagSettings } from "../model/RagModel";
import { useRagErrorToast } from "../hooks/useRagErrorToast";

/** Shared RAG configuration used when creating a knowledge base and editing its corpus. */
export function RagSettingsFields({ settings, setSettings, disabled = false, idPrefix = "rag", className = "" }: {
  settings: RagSettings; setSettings: (settings: RagSettings) => void;
  knowledgeBase?: boolean; disabled?: boolean; idPrefix?: string; className?: string;
}) {
  const { t } = useTranslation("pages");
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const models = useQuery({ queryKey: ["rag-provider-models", user], queryFn: ({ signal }) => fetchModelSelectOptions("", signal) });
  const providerCatalog = useQuery({ queryKey: ["rag-person-ai-catalog", user], queryFn: ({ signal }) => fetchProviderCatalog("PersonAi", signal) });
  const routes = useQuery({ queryKey: ["rag-automatic-routers", user], queryFn: ({ signal }) => fetchAutomaticRouters(signal) });
  useRagErrorToast(routes.error?.message);
  useRagErrorToast(models.error?.message);
  useRagErrorToast(providerCatalog.error?.message);
  const embeddingCatalog = new Map<string, { name: string; instances: number }>();
  for (const model of providerCatalog.data ?? []) {
    if (model.catalogModelType !== "Embedding" || model.status !== "Available" || !model.modelId || (model.dimension != null && model.dimension !== 1024)) continue;
    const entry = embeddingCatalog.get(model.modelCode);
    if (entry) entry.instances += 1;
    else embeddingCatalog.set(model.modelCode, { name: model.modelName, instances: 1 });
  }
  const fieldLabel = (key: string, helpKey: string, id: string) => <div className="flex items-center gap-1.5">
    <FieldLabel htmlFor={id}>{t(`ragUnified.${key}`)}</FieldLabel>
    <InfoTooltip ariaLabel={t("ragUnified.fieldHelp", { field: t(`ragUnified.${key}`) })}>{t(`ragUnified.${helpKey}`)}</InfoTooltip>
  </div>;
  return <div className={`grid gap-4 ${className}`}>
          {(["fulltext", "vector", "graph"] as const).map((key) => <Field key={key}>
            {fieldLabel(key, `${key}Help`, `${idPrefix}-${key}`)}
            <SelectInput disabled={disabled} id={`${idPrefix}-${key}`} aria-label={t(`ragUnified.${key}`)} value={settings[key] ? "enabled" : "disabled"} onValueChange={(value) => setSettings({ ...settings, [key]: value === "enabled" })}>
              <SelectItem value="enabled">{t("ragUnified.enabled")}</SelectItem>
              <SelectItem value="disabled">{t("ragUnified.disabled")}</SelectItem>
            </SelectInput>
          </Field>)}
          <Field>{fieldLabel("embeddingDimension", "dimensionHelp", `${idPrefix}-dimension`)}<Input id={`${idPrefix}-dimension`} type="text" inputMode="numeric" value={1024} disabled /></Field>
          {(["embeddingModelCode", "graphModelCode", "rerankModelCode"] as const).map((key) => <Field key={key}>{fieldLabel(key, `${key}Help`, `${idPrefix}-${key}`)}<SelectInput disabled={disabled} id={`${idPrefix}-${key}`} aria-label={t(`ragUnified.${key}`)} value={settings[key] ?? "__none__"} onValueChange={(value) => setSettings({ ...settings, [key]: value === "__none__" ? null : String(value) })}>
            <SelectItem value="__none__">{t(key === "rerankModelCode" ? "ragUnified.rerankOff" : "ragUnified.chooseModel")}</SelectItem>
            <SelectItem value={autoModelRouteCode(key === "embeddingModelCode" ? "EMBEDDING" : key === "graphModelCode" ? "TEXT" : "RERANK")}>
              {t(autoModelRoutes[key === "embeddingModelCode" ? "EMBEDDING" : key === "graphModelCode" ? "TEXT" : "RERANK"].labelKey)}
            </SelectItem>
            {key !== "graphModelCode" && [...new Map((routes.data?.find(route => route.kind === (key === "embeddingModelCode" ? "Embedding" : "Rerank"))?.candidates ?? []).filter(model => model.modelCode !== model.bindingId).map(model => [model.modelCode, model])).values()].map(model => <SelectItem key={`logical-${model.modelCode}`} value={model.modelCode}>{model.modelName} · {model.modelCode}</SelectItem>)}
            {key === "embeddingModelCode" ? [...embeddingCatalog].map(([code, model]) => <SelectItem key={`person-ai-${code}`} value={`PERSON_AI:EMBEDDING:${code}`}>{model.name} · Person-AI{model.instances > 1 ? ` (${t("ragUnified.instanceCount", { count: model.instances })})` : ""}</SelectItem>) : null}
            {key === "rerankModelCode" ? (providerCatalog.data ?? []).filter((model) => model.catalogModelType === "Rerank" && model.status === "Available" && model.modelId).map((model) => <SelectItem key={`person-ai-rerank-${model.modelId}`} value={`PERSON_AI:RERANK:${model.modelId}`}>{model.modelName} · Person-AI</SelectItem>) : null}
            {(models.data ?? []).filter((model) => model.enabled && !model.modelCode.startsWith("AUTO:") && model.modelSelection?.kind === "Fixed" && (key === "embeddingModelCode" ? ["TEXT_EMBEDDING","MULTIMODAL_EMBEDDING"] : key === "graphModelCode" ? ["TEXT"] : ["TEXT_RERANK","MULTIMODAL_RERANK"]).includes(model.modelType)).map((model) => <SelectItem key={model.modelCode} value={model.modelCode}>{model.displayName}</SelectItem>)}
          </SelectInput>{key === "embeddingModelCode" && (settings.vector || settings.graph) && !settings.embeddingModelCode ? <p className="text-sm text-destructive">{t("ragUnified.embeddingModelRequired")}</p> : null}</Field>)}
  </div>;

}
