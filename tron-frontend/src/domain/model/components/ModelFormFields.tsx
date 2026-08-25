import {
  useRef,
  type RefObject,
} from "react";
import { useTranslation } from "react-i18next";

import {
  Field,
  SelectInput,
  TextArea,
  TextInput,
  ToggleSwitch,
} from "@/shared";

import {
  getDefaultModelCapabilitiesForType,
  getDefaultApiPathForProtocol,
  formatModelCapability,
  getModelCapabilityGroups,
  getModelProtocolOptionValue,
  getModelProtocolOptions,
  getModelTypeFromCapability,
  getPrimaryModelTypeFromCapabilities,
  getValidModelTypesForProtocol,
  normalizeModelCapabilities,
  normalizeReasoningEfforts,
  reasoningEffortOptions,
} from "@/domain/model/constants/modelPageOptions";
import type { ModelMutationPayload } from "@/domain/model/api";
import type {
  ModelCapability,
  ModelProtocol,
  ModelRecord,
  ModelType,
  ReasoningEffort,
} from "@/domain/model/model";
import { SelectGroup, SelectGroupLabel, SelectItem } from "@/components/ui/select";

const MASKED_API_KEY_VALUE = "*****";
const DEFAULT_TEMPERATURE = "0.2";
const DEFAULT_MAX_TOKENS = "4096";
const DEFAULT_TOP_P = "1";
const DEFAULT_MAX_RETRIES = "1";

export interface ModelFormState {
  apiKeyRef: string;
  apiPath: string;
  baseUrl: string;
  description: string;
  displayName: string;
  contextWindowTokens: string;
  maxRetries: string;
  maxTokens: string;
  modelName: string;
  modelCapabilities: ModelCapability[];
  modelType: ModelType;
  protocol: ModelMutationPayload["protocol"];
  supportedReasoningEfforts: ReasoningEffort[];
  defaultReasoningEffort: ReasoningEffort | null;
  structuredOutputEnabled: boolean;
  responsesStatefulEnabled: boolean;
  temperature: string;
  thinkingEnabled: boolean;
  topP: string;
  vendor: string;
}

export type ModelFormFieldKey = "displayName" | "modelName" | "vendor" | "baseUrl" | "apiPath" | "apiKeyRef" | "contextWindowTokens";
export type ModelFormFieldErrors = Partial<Record<ModelFormFieldKey, string>>;

export function emptyModelFormState(): ModelFormState {
  const protocol: ModelFormState["protocol"] = "RESPONSES";
  const modelType: ModelType = "TEXT";
  return {
    apiKeyRef: "",
    apiPath: getDefaultApiPathForProtocol(protocol, modelType),
    baseUrl: "",
    description: "",
    displayName: "",
    contextWindowTokens: "",
    maxRetries: DEFAULT_MAX_RETRIES,
    maxTokens: "",
    modelName: "",
    modelCapabilities: getDefaultModelCapabilitiesForType(modelType),
    modelType,
    protocol,
    supportedReasoningEfforts: ["Low", "Medium", "High"],
    defaultReasoningEffort: "Medium",
    structuredOutputEnabled: true,
    responsesStatefulEnabled: false,
    temperature: DEFAULT_TEMPERATURE,
    thinkingEnabled: true,
    topP: DEFAULT_TOP_P,
    vendor: "",
  };
}

function maskApiKeyValue(value: string) {
  return value.trim() ? MASKED_API_KEY_VALUE : "";
}

export function modelFormFromRecord(record: ModelRecord): ModelFormState {
  return {
    apiKeyRef: maskApiKeyValue(record.apiKeyRef),
    apiPath: record.apiPath ?? "",
    baseUrl: record.baseUrl,
    description: record.description ?? "",
    displayName: record.displayName,
    contextWindowTokens: record.contextWindowTokens === null ? "" : String(record.contextWindowTokens),
    maxRetries: record.maxRetries === null ? DEFAULT_MAX_RETRIES : String(record.maxRetries),
    maxTokens: record.structuredOutputEnabled
      ? ""
      : (record.maxTokens === null ? DEFAULT_MAX_TOKENS : String(record.maxTokens)),
    modelName: record.modelName,
    modelCapabilities: normalizeModelCapabilities(record.modelCapabilities, record.modelType),
    modelType: record.modelType,
    protocol: record.protocol,
    supportedReasoningEfforts: normalizeReasoningEfforts(record.supportedReasoningEfforts),
    defaultReasoningEffort: record.defaultReasoningEffort,
    structuredOutputEnabled: record.structuredOutputEnabled,
    responsesStatefulEnabled: record.responsesStatefulEnabled,
    temperature: record.temperature === null ? DEFAULT_TEMPERATURE : String(record.temperature),
    thinkingEnabled: record.thinkingEnabled,
    topP: record.topP === null ? DEFAULT_TOP_P : String(record.topP),
    vendor: record.vendor,
  };
}

function parseOptionalInteger(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? Math.trunc(parsed) : null;
}

function parseOptionalNumber(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : null;
}

export function modelFormToPayload(
  state: ModelFormState,
  apiKeyRef: string,
  options: { allowInheritance?: boolean; enabled: boolean },
): ModelMutationPayload {
  const modelCapabilities = normalizeModelCapabilities(state.modelCapabilities, state.modelType);
  const modelType = getPrimaryModelTypeFromCapabilities(
    state.protocol,
    modelCapabilities,
    state.modelType,
  );
  const supportedReasoningEfforts = state.thinkingEnabled && modelType === "TEXT"
    ? normalizeReasoningEfforts(state.supportedReasoningEfforts)
    : [];
  const defaultReasoningEffort = state.defaultReasoningEffort
    && supportedReasoningEfforts.includes(state.defaultReasoningEffort)
    ? state.defaultReasoningEffort
    : null;
  const isTextModel = modelType === "TEXT";
  return {
    displayName: state.displayName.trim(),
    vendor: state.vendor.trim(),
    protocol: state.protocol,
    modelType,
    modelCapabilities,
    baseUrl: state.baseUrl.trim(),
    apiPath: state.apiPath.trim() ? state.apiPath.trim() : null,
    apiKeyRef: apiKeyRef.trim(),
    modelName: state.modelName.trim(),
    enabled: options.enabled,
    description: state.description.trim() ? state.description.trim() : null,
    thinkingEnabled: state.thinkingEnabled,
    supportedReasoningEfforts,
    defaultReasoningEffort,
    structuredOutputEnabled: state.structuredOutputEnabled,
    responsesStatefulEnabled: state.protocol === "RESPONSES" ? state.responsesStatefulEnabled : false,
    temperature: isTextModel ? parseOptionalNumber(state.temperature) : null,
    maxTokens: isTextModel ? parseOptionalInteger(state.maxTokens) : null,
    contextWindowTokens: isTextModel ? parseOptionalInteger(state.contextWindowTokens) : null,
    topP: isTextModel ? parseOptionalNumber(state.topP) : null,
    maxRetries: isTextModel ? parseOptionalInteger(state.maxRetries) : null,
    allowInheritance: options.allowInheritance,
  };
}

export function ModelFormFields({
  apiKeyFieldHint,
  apiKeyInputRef,
  apiKeyValue,
  onApiKeyChange,
  apiKeyPlaceholder,
  displayNameError,
  fieldErrors,
  fixedProtocol,
  allowedProtocols,
  allowedCapabilityGroups,
  hideDescription = false,
  onApiKeyDirtyChange,
  readOnly,
  syncModelNameFromDisplayName,
  state,
  onChange,
}: {
  apiKeyFieldHint?: string;
  apiKeyInputRef?: RefObject<HTMLInputElement | null>;
  apiKeyValue?: string;
  onApiKeyChange?: (value: string) => void;
  apiKeyPlaceholder?: string;
  displayNameError?: string;
  fieldErrors?: ModelFormFieldErrors;
  fixedProtocol?: ModelProtocol;
  allowedProtocols?: ModelProtocol[];
  allowedCapabilityGroups?: string[];
  hideDescription?: boolean;
  onApiKeyDirtyChange?: (dirty: boolean) => void;
  readOnly?: boolean;
  syncModelNameFromDisplayName?: boolean;
  state: ModelFormState;
  onChange: (nextState: ModelFormState) => void;
}) {
  const modelNameDirtyRef = useRef(false);

  function patch<K extends keyof ModelFormState>(key: K, value: ModelFormState[K]) {
    onChange({
      ...state,
      [key]: value,
    });
  }

  function patchDisplayName(value: string) {
    onChange({
      ...state,
      displayName: value,
      modelName: syncModelNameFromDisplayName && !modelNameDirtyRef.current
        ? value
        : state.modelName,
    });
  }

  function patchModelName(value: string) {
    if (syncModelNameFromDisplayName) {
      modelNameDirtyRef.current = true;
    }
    patch("modelName", value);
  }

  function defaultMaxTokensValue() {
    return state.structuredOutputEnabled
      ? ""
      : (state.maxTokens.trim() ? state.maxTokens : DEFAULT_MAX_TOKENS);
  }

  const { t } = useTranslation(["common", "pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:modelRegistry.${key}`, { defaultValue, ...options })
  );
  const protocolOptions = getModelProtocolOptions().filter((option) => (
    allowedProtocols === undefined || allowedProtocols.includes(option.protocol)
  ));
  const modelCapabilityGroups = getModelCapabilityGroups(state.protocol).filter((group) => (
    allowedCapabilityGroups === undefined || allowedCapabilityGroups.includes(group.label)
  ));
  const modelCapabilityDisplayValue = normalizeModelCapabilities(
    state.modelCapabilities,
    state.modelType,
  )
    .map((value) => pageT(`capabilities.${value}`, formatModelCapability(value)))
    .join(", ");
  const maxTokensDisabled = Boolean(readOnly || state.structuredOutputEnabled);
  const isTextModel = state.modelType === "TEXT";
  const reasoningFieldsDisabled = Boolean(
    readOnly || !state.thinkingEnabled || state.modelType !== "TEXT",
  );

  return (
    <div className="grid gap-4">
      <div className="grid gap-4 md:grid-cols-2">
        <Field required label={pageT("form.displayName", "Display Name")} error={displayNameError ?? fieldErrors?.displayName}>
          <TextInput
            value={state.displayName}
            aria-invalid={Boolean(displayNameError ?? fieldErrors?.displayName)}
            onChange={(event) => patchDisplayName(event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field required label={pageT("form.vendorModelId", "Vendor Model ID")} error={fieldErrors?.modelName}>
          <TextInput
            value={state.modelName}
            aria-invalid={Boolean(fieldErrors?.modelName)}
            onChange={(event) => patchModelName(event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field required label={pageT("form.vendor", "Vendor")} error={fieldErrors?.vendor}>
          <TextInput
            value={state.vendor}
            aria-invalid={Boolean(fieldErrors?.vendor)}
            onChange={(event) => patch("vendor", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field required label={pageT("form.protocol", "Protocol")}>
          <SelectInput
            value={fixedProtocol ?? getModelProtocolOptionValue(state.protocol, state.modelType)}
            onValueChange={(event) => {
              const selectedOption = protocolOptions.find((option) => option.value === event);
              if (!selectedOption) return;
              const nextProtocol = selectedOption.protocol;
              const nextModelType = selectedOption.modelType;
              onChange({
                ...state,
                apiPath: getDefaultApiPathForProtocol(nextProtocol, nextModelType),
                maxRetries: state.maxRetries.trim() ? state.maxRetries : DEFAULT_MAX_RETRIES,
                maxTokens: defaultMaxTokensValue(),
                modelCapabilities: getDefaultModelCapabilitiesForType(nextModelType),
                protocol: nextProtocol,
                modelType: nextModelType,
                responsesStatefulEnabled: false,
                temperature: state.temperature.trim() ? state.temperature : DEFAULT_TEMPERATURE,
                topP: state.topP.trim() ? state.topP : DEFAULT_TOP_P,
              });
            }}
            disabled={readOnly || fixedProtocol !== undefined}
          >
            {protocolOptions.map((item) => (
              <SelectItem key={item.value} value={item.value}>
                {pageT(
                  `protocol.${item.value === "OPENAI_COMPATIBLE" ? "openaiCompatible.chat" : item.value === "OPENAI_COMPATIBLE_EMBEDDING" ? "openaiCompatible.embedding" : item.value === "RESPONSES" ? "responses" : "rerank"}`,
                  item.label,
                )}
              </SelectItem>
            ))}
          </SelectInput>
        </Field>
        <Field required label={pageT("form.modelType", "Model Type")}>
          <SelectInput
            value={state.modelCapabilities}
            displayValue={modelCapabilityDisplayValue}
            multiple
            onValueChange={(event) => {
              const nextModelCapabilities = Array.from(event, (option) => (
                option as ModelCapability
              ));
              const nextModelType = getPrimaryModelTypeFromCapabilities(
                state.protocol,
                nextModelCapabilities,
                state.modelType,
              );
              onChange({
                ...state,
                apiPath: getDefaultApiPathForProtocol(state.protocol, nextModelType),
                maxRetries: state.maxRetries.trim() ? state.maxRetries : DEFAULT_MAX_RETRIES,
                maxTokens: defaultMaxTokensValue(),
                modelCapabilities: normalizeModelCapabilities(nextModelCapabilities, nextModelType),
                modelType: nextModelType,
                temperature: state.temperature.trim() ? state.temperature : DEFAULT_TEMPERATURE,
                topP: state.topP.trim() ? state.topP : DEFAULT_TOP_P,
              });
            }}
            disabled={readOnly}
          >
            {modelCapabilityGroups.map((group) => (
              <SelectGroup key={group.label} >
                <SelectGroupLabel>{pageT(`capabilityGroups.${group.label}`, group.label)}</SelectGroupLabel>
                {
                  group.options.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {pageT(`capabilities.${item.value}`, item.label)}
                    </SelectItem>
                  ))
                }
              </SelectGroup>
            ))}
          </SelectInput>
        </Field>
        <Field required label={pageT("form.baseUrl", "Base URL")} error={fieldErrors?.baseUrl}>
          <TextInput
            value={state.baseUrl}
            aria-invalid={Boolean(fieldErrors?.baseUrl)}
            onChange={(event) => patch("baseUrl", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field required label={pageT("form.apiPath", "API Path")} error={fieldErrors?.apiPath}>
          <TextInput
            value={state.apiPath}
            aria-invalid={Boolean(fieldErrors?.apiPath)}
            onChange={(event) => patch("apiPath", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field required label={pageT("form.apiKey", "API Key")} error={fieldErrors?.apiKeyRef} hint={apiKeyFieldHint}>
          {readOnly ? (
            <TextInput
              value={maskApiKeyValue(state.apiKeyRef)}
              disabled
              readOnly
            />
          ) : apiKeyValue !== undefined ? (
            <TextInput
              ref={apiKeyInputRef}
              type="password"
              value={apiKeyValue}
              aria-invalid={Boolean(fieldErrors?.apiKeyRef)}
              placeholder={apiKeyPlaceholder}
              autoComplete="new-password"
              onChange={(event) => {
                onApiKeyChange?.(event.target.value);
                onApiKeyDirtyChange?.(Boolean(event.target.value.trim()));
              }}
            />
          ) : (
            <TextInput
              ref={apiKeyInputRef}
              type="password"
              defaultValue=""
              aria-invalid={Boolean(fieldErrors?.apiKeyRef)}
              placeholder={maskApiKeyValue(state.apiKeyRef) || apiKeyPlaceholder}
              autoComplete="new-password"
              onChange={(event) => onApiKeyDirtyChange?.(Boolean(event.target.value.trim()))}
            />
          )}
        </Field>
      </div>

      {!hideDescription && (
        <Field label={pageT("form.description", "Description")}>
          <TextArea
            className="min-h-24"
            value={state.description}
            onChange={(event) => patch("description", event.target.value)}
            disabled={readOnly}
          />
        </Field>
      )}

      <div className={isTextModel ? "grid gap-4 md:grid-cols-2 xl:grid-cols-4" : "hidden"}>
        <Field label={pageT("form.temperature", "Temperature")}>
          <TextInput
            type="number"
            step="0.1"
            value={state.temperature}
            onChange={(event) => patch("temperature", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label={pageT("form.maxTokens", "Max Tokens")}>
          <TextInput
            type="number"
            step="1"
            value={state.maxTokens}
            onChange={(event) => patch("maxTokens", event.target.value)}
            disabled={maxTokensDisabled}
          />
        </Field>
        <Field required label={pageT("form.contextWindow", "Context Window")} error={fieldErrors?.contextWindowTokens}>
          <TextInput
            type="number"
            step="1"
            value={state.contextWindowTokens}
            aria-invalid={Boolean(fieldErrors?.contextWindowTokens)}
            onChange={(event) => patch("contextWindowTokens", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label={pageT("form.topP", "Top P")}>
          <TextInput
            type="number"
            step="0.1"
            value={state.topP}
            onChange={(event) => patch("topP", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label={pageT("form.maxRetries", "Max Retries")}>
          <TextInput
            type="number"
            step="1"
            value={state.maxRetries}
            onChange={(event) => patch("maxRetries", event.target.value)}
            disabled={readOnly}
          />
        </Field>
      </div>

      {state.protocol === "RESPONSES" ? (
        <Field label={t("common:responsesState.label", "Responses State")}>
          <SelectInput
            value={state.responsesStatefulEnabled ? "true" : "false"}
            onValueChange={(event) => patch("responsesStatefulEnabled", event === "true")}
            disabled={readOnly}
          >
            <SelectItem value="false">{t("common:responsesState.stateless", "Stateless")}</SelectItem>
            <SelectItem value="true">{t("common:responsesState.stateful", "Stateful")}</SelectItem>
          </SelectInput>
        </Field>
      ) : null}

      <div className={isTextModel ? "grid gap-4 md:grid-cols-2" : "hidden"}>
        <Field
          label={t("common:reasoningEffort.supportedLabel", "Supported reasoning efforts")}
          hint={t(
            "common:reasoningEffort.supportedHint",
            "Configure this per model; providers and model versions support different values.",
          )}
        >
          <SelectInput
            value={state.supportedReasoningEfforts}
            multiple
            onValueChange={(event) => {
              const nextSupported = normalizeReasoningEfforts(
                Array.from(event, (option) => option as ReasoningEffort),
              );
              onChange({
                ...state,
                supportedReasoningEfforts: nextSupported,
                defaultReasoningEffort: state.defaultReasoningEffort
                  && nextSupported.includes(state.defaultReasoningEffort)
                  ? state.defaultReasoningEffort
                  : null,
              });
            }}
            disabled={reasoningFieldsDisabled}
          >
            {reasoningEffortOptions.map((item) => (
              <SelectItem key={item.value} value={item.value}>
                {pageT(`reasoning.${item.value}`, item.label)}
              </SelectItem>
            ))}
          </SelectInput>
        </Field>
        <Field
          label={t("common:reasoningEffort.defaultLabel", "Default reasoning effort")}
          hint={t(
            "common:reasoningEffort.defaultHint",
            "Public chats use this value unless the user chooses an override.",
          )}
        >
          <SelectInput
            value={state.defaultReasoningEffort ?? "__provider_default__"}
            onValueChange={(event) => patch(
              "defaultReasoningEffort",
              event === "__provider_default__" ? null : event as ReasoningEffort,
            )}
            disabled={reasoningFieldsDisabled || state.supportedReasoningEfforts.length === 0}
          >
            <SelectItem value="__provider_default__">
              {t("common:reasoningEffort.providerDefault", "Provider default")}
            </SelectItem>
            {reasoningEffortOptions
              .filter((item) => state.supportedReasoningEfforts.includes(item.value))
              .map((item) => (
                <SelectItem key={item.value} value={item.value}>
                  {pageT(`reasoning.${item.value}`, item.label)}
                </SelectItem>
              ))}
          </SelectInput>
        </Field>
      </div>

      <div className={isTextModel ? "grid gap-3 md:grid-cols-2" : "hidden"}>
        <ToggleSwitch
          checked={state.thinkingEnabled}
          disabled={readOnly}
            label={pageT("form.thinkingEnabled", "Thinking Enabled")}
          onCheckedChange={(checked) => patch("thinkingEnabled", checked)}
        />
        <ToggleSwitch
          checked={state.structuredOutputEnabled}
          disabled={readOnly}
          label={pageT("form.structuredOutputEnabled", "Structured Output Enabled")}
          onCheckedChange={(checked) => onChange({
            ...state,
            maxTokens: checked ? "" : (state.maxTokens.trim() ? state.maxTokens : DEFAULT_MAX_TOKENS),
            structuredOutputEnabled: checked,
          })}
        />
      </div>
    </div>
  );
}
