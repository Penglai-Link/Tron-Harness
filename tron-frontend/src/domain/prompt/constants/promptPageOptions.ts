import type {
  PromptCategory,
  PromptVariableType,
} from "@/domain/prompt/model";

export const promptCategoryOptions: Array<{ label: string; value: PromptCategory }> = [
  { label: "System", value: "SYSTEM" },
  { label: "Developer", value: "DEVELOPER" },
  { label: "Persona", value: "PERSONA" },
  { label: "Agent role", value: "AGENT_ROLE" },
  { label: "Tool", value: "TOOL" },
  { label: "Template", value: "TEMPLATE" },
  { label: "Action", value: "ACTION" },
];

export const promptFilterCategoryOptions = promptCategoryOptions.filter(
  (option) => option.value !== "PERSONA",
);

export const promptVariableTypeOptions: Array<{ label: string; value: PromptVariableType }> = [
  { label: "STRING", value: "STRING" },
  { label: "NUMBER", value: "NUMBER" },
  { label: "BOOLEAN", value: "BOOLEAN" },
  { label: "JSON", value: "JSON" },
];

export function promptCategoryLabel(category: PromptCategory) {
  return promptCategoryOptions.find((option) => option.value === category)?.label ?? category;
}
