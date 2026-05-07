export interface ModelCostLine {
  label: string;
  value: string;
}

export interface ModelRegistryItem {
  id: string;
  name: string;
  purpose: string;
  vendor: "OpenAI" | "Anthropic" | "Google";
  context: string;
  inheritance: string;
  status: "Enabled" | "Disabled";
  price: string;
  maxOutput: string;
  trainingData: string;
  speed: string;
  benchmarks: Array<{ label: string; score: number }>;
}
