import { Field, SelectInput, TextArea, TextInput, ToggleSwitch } from "@/components/forms";
import { useTranslation } from "react-i18next";

import type {
  MemoryMetadataItem,
  MemoryRecord,
  MemoryReviewStatus,
  MemoryScopeKind,
  MemorySearchTarget,
  MemoryStatus,
  MemoryType,
} from "@/domain/memory/model";
import { SelectItem } from "@/components/ui/select";

export type MemoryTargetFormValue = "space:current" | "user:current" | "domain";

export interface MemoryFormState {
  injectable: boolean;
  memoryStatus: MemoryStatus;
  memoryType: MemoryType;
  metadataText: string;
  reviewStatus: MemoryReviewStatus;
  semanticKey: string;
  sourceLabel: string;
  tagsText: string;
  targetId: string;
  text: string;
  title: string;
}

export interface MemoryMutationInput {
  domainCode?: string;
  injectable: boolean;
  memoryStatus: MemoryStatus;
  memoryType: MemoryType;
  metadata: MemoryMetadataItem[];
  reviewStatus: MemoryReviewStatus;
  scope: MemoryScopeKind;
  semanticKey: string;
  sourceLabel: string;
  tags: string[];
  targetCode: string;
  targetId: string;
  targetLabel: string;
  text: string;
  title: string;
}

export const memoryTypeOptions: MemoryType[] = [
  "FACT",
  "PROCEDURE",
  "PROJECT_DECISION",
  "RISK",
  "SPACE_RULE",
  "USER_PREFERENCE",
  "WORK_DOMAIN_EXPERIENCE",
];

export const memoryStatusValues: MemoryStatus[] = [
  "ACTIVE",
  "CANDIDATE",
  "ARCHIVED",
  "DISABLED",
];

export const memoryReviewStatusValues: MemoryReviewStatus[] = [
  "PENDING_REVIEW",
  "APPROVED",
  "REJECTED",
];

export function emptyMemoryFormState(targetId = "space:current"): MemoryFormState {
  return {
    injectable: false,
    memoryStatus: "ACTIVE",
    memoryType: "FACT",
    metadataText: "",
    reviewStatus: "PENDING_REVIEW",
    semanticKey: "",
    sourceLabel: "Manual entry",
    tagsText: "",
    targetId,
    text: "",
    title: "",
  };
}

export function memoryFormFromRecord(record: MemoryRecord): MemoryFormState {
  return {
    injectable: record.injectable,
    memoryStatus: record.memoryStatus,
    memoryType: record.memoryType,
    metadataText: formatMetadata(record.metadata),
    reviewStatus: record.reviewStatus,
    semanticKey: record.semanticKey,
    sourceLabel: record.sourceLabel,
    tagsText: record.tags.join(", "),
    targetId: record.targetId,
    text: record.text,
    title: record.title,
  };
}

function formatMetadata(metadata: MemoryMetadataItem[]) {
  return metadata.map((item) => `${item.label}: ${item.value}`).join("\n");
}

function parseMetadata(value: string): MemoryMetadataItem[] {
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const separatorIndex = line.indexOf(":");
      if (separatorIndex === -1) {
        return { label: line, value: "" };
      }
      return {
        label: line.slice(0, separatorIndex).trim(),
        value: line.slice(separatorIndex + 1).trim(),
      };
    })
    .filter((item) => item.label || item.value);
}

function parseTags(value: string) {
  const seen = new Set<string>();
  return value
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean)
    .filter((tag) => {
      const normalized = tag.toLowerCase();
      if (seen.has(normalized)) return false;
      seen.add(normalized);
      return true;
    });
}

export function validateMemoryForm(state: MemoryFormState, target?: MemorySearchTarget) {
  if (!state.title.trim()) return "Title is required";
  if (!state.text.trim()) return "Memory text is required";
  if (!target) return "Target is required";
  if (state.targetId === "domain" && target.scope !== "domain") return "Work domain is required";
  const metadata = parseMetadata(state.metadataText);
  if (metadata.some((item) => !item.label && item.value)) return "Metadata label is required";
  return "";
}

export function memoryFormToInput(state: MemoryFormState, target: MemorySearchTarget): MemoryMutationInput {
  return {
    domainCode: target.scope === "domain" ? target.code : undefined,
    injectable: state.injectable,
    memoryStatus: state.memoryStatus,
    memoryType: state.memoryType,
    metadata: parseMetadata(state.metadataText),
    reviewStatus: state.reviewStatus,
    scope: target.scope,
    semanticKey: state.semanticKey.trim(),
    sourceLabel: state.sourceLabel.trim() || "Manual entry",
    tags: parseTags(state.tagsText),
    targetCode: target.code,
    targetId: target.id,
    targetLabel: target.label,
    text: state.text.trim(),
    title: state.title.trim(),
  };
}

export function MemoryFormFields({
  domainTargets,
  lockedTarget,
  mode,
  onChange,
  state,
  targetOptions,
}: {
  domainTargets: MemorySearchTarget[];
  lockedTarget?: MemorySearchTarget;
  mode: "create" | "edit";
  onChange: (nextState: MemoryFormState) => void;
  state: MemoryFormState;
  targetOptions: MemorySearchTarget[];
}) {
  const { t } = useTranslation(["pages"]);
  const text = (key: string, fallback: string) => t(`pages:memory.form.${key}`, { defaultValue: fallback });
  const selectedTarget = targetOptions.find((target) => target.id === state.targetId);
  const targetValue: MemoryTargetFormValue = selectedTarget?.scope === "domain" || state.targetId.startsWith("domain")
    ? "domain"
    : (state.targetId as MemoryTargetFormValue);
  const firstDomainId = domainTargets[0]?.id ?? "";
  const selectedDomainId = selectedTarget?.scope === "domain"
    ? selectedTarget.id
    : "";

  function patch<K extends keyof MemoryFormState>(key: K, value: MemoryFormState[K]) {
    onChange({
      ...state,
      [key]: value,
    });
  }

  return (
    <div className="grid gap-5">
      {mode === "create" ? (
        <div className="grid gap-4 md:grid-cols-2">
          <Field label={t("pages:memory.filters.scope")}>
            <SelectInput
              value={targetValue}
              onValueChange={(event) => {
                const nextValue = event as MemoryTargetFormValue;
                if (nextValue === "domain") {
                  patch("targetId", selectedDomainId || firstDomainId || "domain");
                  return;
                }
                patch("targetId", nextValue);
              }}
            >
              <SelectItem value="space:current">{t("pages:memory.scopes.space")}</SelectItem>
              <SelectItem value="user:current">{t("pages:memory.scopes.user")}</SelectItem>
              <SelectItem value="domain">{t("pages:memory.scopes.domain")}</SelectItem>
            </SelectInput>
          </Field>
          <Field label={text("workDomain", "Work domain")}>
            <SelectInput
              disabled={targetValue !== "domain"}
              value={selectedDomainId}
              onValueChange={(event) => patch("targetId", event!)}
            >
              <SelectItem value="">{targetValue === "domain" ? t("pages:memory.filters.domain") : text("notApplicable", "Not applicable")}</SelectItem>
              {domainTargets.map((target) => (
                <SelectItem key={target.id} value={target.id}>{target.label}</SelectItem>
              ))}
            </SelectInput>
          </Field>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          <Field label={text("scope", "Scope")}>
            <TextInput value={lockedTarget?.scope ?? ""} disabled readOnly />
          </Field>
          <Field label={text("target", "Target")}>
            <TextInput value={lockedTarget?.label ?? ""} disabled readOnly />
          </Field>
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        <div className="md:col-span-2">
          <Field label={text("title", "Title")}>
            <TextInput
              value={state.title}
              onChange={(event) => patch("title", event.target.value)}
            />
          </Field>
        </div>
        <div className="md:col-span-2">
          <Field label={text("text", "Memory text")}>
            <TextArea
              className="min-h-40"
              value={state.text}
              onChange={(event) => patch("text", event.target.value)}
            />
          </Field>
        </div>
        <Field label={text("type", "Memory type")}>
          <SelectInput
            value={state.memoryType}
            onValueChange={(event) => patch("memoryType", event as MemoryType)}
          >
            {memoryTypeOptions.map((value) => (
              <SelectItem key={value} value={value}>{value}</SelectItem>
            ))}
          </SelectInput>
        </Field>
        <Field label={text("status", "Memory status")}>
          <SelectInput
            value={state.memoryStatus}
            onValueChange={(event) => patch("memoryStatus", event as MemoryStatus)}
          >
            {memoryStatusValues.map((value) => (
              <SelectItem key={value} value={value}>{value}</SelectItem>
            ))}
          </SelectInput>
        </Field>
        <Field label={text("reviewStatus", "Review status")}>
          <SelectInput
            value={state.reviewStatus}
            onValueChange={(event) => patch("reviewStatus", event as MemoryReviewStatus)}
          >
            {memoryReviewStatusValues.map((value) => (
              <SelectItem key={value} value={value}>{value}</SelectItem>
            ))}
          </SelectInput>
        </Field>
        <Field label={text("use", "Use")}>
          <ToggleSwitch
            checked={state.injectable}
            label={state.injectable ? text("autoContext", "Auto context") : text("searchOnly", "Search only")}
            onCheckedChange={(checked) => patch("injectable", checked)}
          />
        </Field>
        <Field label={text("sourceLabel", "Source label")}>
          <TextInput
            value={state.sourceLabel}
            onChange={(event) => patch("sourceLabel", event.target.value)}
          />
        </Field>
        <Field label={text("semanticKey", "Semantic key")}>
          <TextInput
            value={state.semanticKey}
            placeholder={text("optional", "Optional")}
            onChange={(event) => patch("semanticKey", event.target.value)}
          />
        </Field>
        <div className="md:col-span-2">
          <Field label={text("tags", "Tags")} hint={text("tagsHint", "Comma-separated values.")}>
            <TextInput
              value={state.tagsText}
              onChange={(event) => patch("tagsText", event.target.value)}
            />
          </Field>
        </div>
        <div className="md:col-span-2">
          <Field label={text("metadata", "Metadata rows")} hint={text("metadataHint", "One key-value pair per line, formatted as Key: Value.")}>
            <TextArea
              className="min-h-28"
              value={state.metadataText}
              onChange={(event) => patch("metadataText", event.target.value)}
            />
          </Field>
        </div>
      </div>
    </div>
  );
}
