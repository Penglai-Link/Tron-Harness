import type {
  PromptCategory,
  PromptSourceKind,
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

export function promptCategoryLabel(category: PromptCategory) {
  return promptCategoryOptions.find((option) => option.value === category)?.label ?? category;
}
