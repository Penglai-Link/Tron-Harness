import type { ReasoningEffort } from "@/domain/model/model";

export const reasoningEffortOptions: Array<{ label: string; value: ReasoningEffort }> = [
  { label: "None", value: "None" },
  { label: "Minimal", value: "Minimal" },
  { label: "Low", value: "Low" },
  { label: "Medium", value: "Medium" },
  { label: "High", value: "High" },
  { label: "Extra High", value: "XHigh" },
  { label: "Max", value: "Max" },
  { label: "Ultra", value: "Ultra" },
];

export function normalizeReasoningEfforts(
  values: ReasoningEffort[] | null | undefined,
): ReasoningEffort[] {
  const selected = new Set(Array.isArray(values) ? values : []);
  return reasoningEffortOptions
    .map((option) => option.value)
    .filter((value) => selected.has(value));
}

export function formatReasoningEffort(value: ReasoningEffort) {
  return reasoningEffortOptions.find((option) => option.value === value)?.label ?? value;
}
