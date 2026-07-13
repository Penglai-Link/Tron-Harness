import type {
  PromptCategory,
  PromptSourceKind,
  PromptVariableType,
} from "@/domain/prompt/model";

export const promptCategoryOptions: Array<{ label: string; value: PromptCategory }> = [
  { label: "System", value: "SYSTEM" },
  { label: "Persona", value: "PERSONA" },
  { label: "Tool", value: "TOOL" },
  { label: "Template", value: "TEMPLATE" },
];

export const promptSourceOptions: Array<{ label: string; value: PromptSourceKind }> = [
  { label: "Local", value: "LOCAL" },
  { label: "Inherited", value: "INHERITED" },
  { label: "Override", value: "OVERRIDE" },
];

export const promptVariableTypeOptions: Array<{ label: string; value: PromptVariableType }> = [
  { label: "STRING", value: "STRING" },
  { label: "NUMBER", value: "NUMBER" },
  { label: "BOOLEAN", value: "BOOLEAN" },
  { label: "JSON", value: "JSON" },
];

export function promptCategoryLabel(category: PromptCategory) {
  return promptCategoryOptions.find((option) => option.value === category)?.label ?? category;
}
