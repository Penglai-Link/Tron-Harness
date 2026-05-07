import {
  getPromptVariables,
  listPrompts,
} from "@/domain/prompt/mock/data";
import { useServiceQuery } from "@/domain/service-config";
export type {
  StatusTone
} from "@/domain/prompt/model/enum";
export type {
  PromptVariable,
  PromptRecord
} from "@/domain/prompt/model/model";

export const promptApiEndpoints = {
  listPrompts: "GET /api/v1/prompts",
  listPromptVariables: "GET /api/v1/prompts/variables",
} as const;

export function usePromptVariables() {
  return useServiceQuery({
    domain: "prompt",
    endpoint: promptApiEndpoints.listPromptVariables,
    mockData: () => getPromptVariables(),
    deps: [],
  });
}

export function usePrompts() {
  return useServiceQuery({
    domain: "prompt",
    endpoint: promptApiEndpoints.listPrompts,
    mockData: () => listPrompts(),
    deps: [],
  });
}
