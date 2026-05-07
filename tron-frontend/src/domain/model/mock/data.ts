import type {
  ModelCostLine,
  ModelRegistryItem,
} from "@/domain/model/model/ModelRegistryModel";

export function getModelCostPreview(): ModelCostLine[] {
  return [
    { label: "Input (1M Tokens)", value: "$10.00" },
    { label: "Output (1M Tokens)", value: "$30.00" },
    { label: "Context Window", value: "128,000" },
  ];
}

export function getModels(): ModelRegistryItem[] {
  return [
    {
      id: "gpt-4-turbo",
      name: "gpt-4-turbo",
      purpose: "Default Reasoning",
      vendor: "OpenAI",
      context: "128k tokens",
      inheritance: "Global",
      status: "Enabled",
      price: "$10.00 / 1M Input",
      maxOutput: "4,096 tokens",
      trainingData: "Dec 2023",
      speed: "~42 tk/s",
      benchmarks: [
        { label: "MMLU (Reasoning)", score: 88.7 },
        { label: "HumanEval (Coding)", score: 87.1 },
        { label: "GSM8K (Math)", score: 95.3 },
      ],
    },
    {
      id: "claude-3-opus",
      name: "claude-3-opus-20240229",
      purpose: "Complex Tasks",
      vendor: "Anthropic",
      context: "200k tokens",
      inheritance: "Workspace A",
      status: "Enabled",
      price: "$15.00 / 1M Input",
      maxOutput: "4,096 tokens",
      trainingData: "Aug 2023",
      speed: "~31 tk/s",
      benchmarks: [
        { label: "MMLU (Reasoning)", score: 86.8 },
        { label: "HumanEval (Coding)", score: 84.9 },
        { label: "GSM8K (Math)", score: 94.2 },
      ],
    },
    {
      id: "gpt-3.5-turbo",
      name: "gpt-3.5-turbo",
      purpose: "Legacy Fast",
      vendor: "OpenAI",
      context: "16k tokens",
      inheritance: "Global",
      status: "Disabled",
      price: "$0.50 / 1M Input",
      maxOutput: "2,048 tokens",
      trainingData: "Sep 2021",
      speed: "~91 tk/s",
      benchmarks: [
        { label: "MMLU (Reasoning)", score: 70.1 },
        { label: "HumanEval (Coding)", score: 48.1 },
        { label: "GSM8K (Math)", score: 57.1 },
      ],
    },
  ];
}
