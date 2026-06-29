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
  getModelCapabilityGroups,
  getModelProtocolOptions,
  getModelTypeFromCapability,
  getPrimaryModelTypeFromCapabilities,
  getValidModelTypesForProtocol,
  normalizeModelCapabilities,
} from "@/domain/model/constants/modelPageOptions";
import type { ModelMutationPayload } from "@/domain/model/api";
import type { ModelCapability, ModelRecord, ModelType } from "@/domain/model/model";
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
  structuredOutputEnabled: boolean;
  responsesStatefulEnabled: boolean;
  temperature: string;
  thinkingEnabled: boolean;
  topP: string;
  vendor: string;
}

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
    structuredOutputEnabled: state.structuredOutputEnabled,
    responsesStatefulEnabled: state.protocol === "RESPONSES" ? state.responsesStatefulEnabled : false,
    temperature: parseOptionalNumber(state.temperature),
    maxTokens: parseOptionalInteger(state.maxTokens),
    contextWindowTokens: parseOptionalInteger(state.contextWindowTokens),
    topP: parseOptionalNumber(state.topP),
    maxRetries: parseOptionalInteger(state.maxRetries),
    allowInheritance: options.allowInheritance,
  };
}

export function ModelFormFields({
  apiKeyFieldHint,
  apiKeyInputRef,
  apiKeyPlaceholder,
  displayNameError,
  onApiKeyDirtyChange,
  readOnly,
  syncModelNameFromDisplayName,
  state,
  onChange,
}: {
  apiKeyFieldHint?: string;
  apiKeyInputRef?: RefObject<HTMLInputElement | null>;
  apiKeyPlaceholder?: string;
  displayNameError?: string;
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

  const { t } = useTranslation(["common"]);
  const protocolOptions = getModelProtocolOptions(state.modelType);
  const modelCapabilityGroups = getModelCapabilityGroups(state.protocol);
  const maxTokensDisabled = Boolean(readOnly || state.structuredOutputEnabled);

  return (
    <div className="grid gap-4">
      <div className="grid gap-4 md:grid-cols-2">
        <Field label="Display Name" error={displayNameError}>
          <TextInput
            value={state.displayName}
            onChange={(event) => patchDisplayName(event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label="Vendor Model ID">
          <TextInput
            value={state.modelName}
            onChange={(event) => patchModelName(event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label="Vendor">
          <TextInput
            value={state.vendor}
            onChange={(event) => patch("vendor", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label="Protocol">
          <SelectInput
            value={state.protocol}
            onValueChange={(event) => {
              const nextProtocol = event as ModelFormState["protocol"];
              const nextValidModelTypes = getValidModelTypesForProtocol(nextProtocol);
              const nextModelCapabilities = state.modelCapabilities.filter((value) => (
                nextValidModelTypes.includes(getModelTypeFromCapability(value))
              ));
              const nextFallbackModelType = nextValidModelTypes.includes(state.modelType)
                ? state.modelType
                : (nextValidModelTypes[0] ?? state.modelType);
              const nextModelType = getPrimaryModelTypeFromCapabilities(
                nextProtocol,
                nextModelCapabilities,
                nextFallbackModelType,
              );
              onChange({
                ...state,
                apiPath: getDefaultApiPathForProtocol(nextProtocol, nextModelType),
                maxRetries: state.maxRetries.trim() ? state.maxRetries : DEFAULT_MAX_RETRIES,
                maxTokens: defaultMaxTokensValue(),
                modelCapabilities: normalizeModelCapabilities(nextModelCapabilities, nextModelType),
                protocol: nextProtocol,
                modelType: nextModelType,
                responsesStatefulEnabled: false,
                temperature: state.temperature.trim() ? state.temperature : DEFAULT_TEMPERATURE,
                topP: state.topP.trim() ? state.topP : DEFAULT_TOP_P,
              });
            }}
            disabled={readOnly}
          >
            {protocolOptions.map((item) => (
              <SelectItem key={item.value} value={item.value}>
                {item.label}
              </SelectItem>
            ))}
          </SelectInput>
        </Field>
        <Field label="Model Type">
          <SelectInput
            value={state.modelCapabilities}
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
                <SelectGroupLabel>{group.label}</SelectGroupLabel>
                {
                  group.options.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {item.label}
                    </SelectItem>
                  ))
                }
              </SelectGroup>
            ))}
          </SelectInput>
        </Field>
        <Field label="Base URL">
          <TextInput
            value={state.baseUrl}
            onChange={(event) => patch("baseUrl", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label="API Path">
          <TextInput
            value={state.apiPath}
            onChange={(event) => patch("apiPath", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label="API Key" hint={apiKeyFieldHint}>
          {readOnly ? (
            <TextInput
              value={maskApiKeyValue(state.apiKeyRef)}
              disabled
              readOnly
            />
          ) : (
            <TextInput
              ref={apiKeyInputRef}
              type="password"
              defaultValue=""
              placeholder={maskApiKeyValue(state.apiKeyRef) || apiKeyPlaceholder}
              autoComplete="new-password"
              onChange={(event) => onApiKeyDirtyChange?.(Boolean(event.target.value.trim()))}
            />
          )}
        </Field>
      </div>

      <Field label="Description">
        <TextArea
          className="min-h-24"
          value={state.description}
          onChange={(event) => patch("description", event.target.value)}
          disabled={readOnly}
        />
      </Field>

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <Field label="Temperature">
          <TextInput
            type="number"
            step="0.1"
            value={state.temperature}
            onChange={(event) => patch("temperature", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label="Max Tokens">
          <TextInput
            type="number"
            step="1"
            value={state.maxTokens}
            onChange={(event) => patch("maxTokens", event.target.value)}
            disabled={maxTokensDisabled}
          />
        </Field>
        <Field label="Context Window">
          <TextInput
            type="number"
            step="1"
            value={state.contextWindowTokens}
            onChange={(event) => patch("contextWindowTokens", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label="Top P">
          <TextInput
            type="number"
            step="0.1"
            value={state.topP}
            onChange={(event) => patch("topP", event.target.value)}
            disabled={readOnly}
          />
        </Field>
        <Field label="Max Retries">
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

      <div className="grid gap-3 md:grid-cols-2">
        <ToggleSwitch
          checked={state.thinkingEnabled}
          disabled={readOnly}
          label="Thinking Enabled"
          onCheckedChange={(checked) => patch("thinkingEnabled", checked)}
        />
        <ToggleSwitch
          checked={state.structuredOutputEnabled}
          disabled={readOnly}
          label="Structured Output Enabled"
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
