import type {
  SystemAgentModelRecord,
  WorkspaceSystemAgentRecord,
} from "@/domain/agent/model";
import type { ModelSelectOption } from "@/domain/model/model/ModelRegistryModel";

export interface AutomationAgentOption {
  agentCode: string;
  description: string;
  models: SystemAgentModelRecord[];
  name: string;
}

export interface AutomationModelOption {
  label: string;
  modelCode: string | null;
  optionCode: string;
}

export const mainAutomationAgentOption: AutomationAgentOption = {
  agentCode: "main-agent",
  description: "Default workspace chat agent.",
  models: [],
  name: "Main Agent",
};

const mainDefaultModelOptionCode = "main:default";

export function isMainAutomationAgent(agentCode: string) {
  return !agentCode.trim() || agentCode.trim() === mainAutomationAgentOption.agentCode;
}

export function availableAutomationAgents(
  systemAgents: WorkspaceSystemAgentRecord[],
): AutomationAgentOption[] {
  const availableSystemAgents = systemAgents
    .filter((agent) => agent.enabled && agent.available && agent.status === "Enabled")
    .map((agent) => ({
      agentCode: agent.agentCode,
      description: agent.description,
      models: agent.models,
      name: agent.name,
    }));

  return [mainAutomationAgentOption, ...availableSystemAgents];
}

export function automationModelOptionsForAgent(
  workspaceModels: ModelSelectOption[],
  agent: AutomationAgentOption,
): AutomationModelOption[] {
  if (isMainAutomationAgent(agent.agentCode)) {
    return [{
      label: "Workspace default text model",
      modelCode: null,
      optionCode: mainDefaultModelOptionCode,
    }, ...workspaceModels
      .filter((model) => isAutomationTextModelOption(model))
      .map((model) => ({
        label: `${model.displayName || model.modelName} (${model.modelName})`,
        modelCode: model.modelCode,
        optionCode: mainAutomationModelOptionCode(model.modelCode),
      }))];
  }

  return agent.models
    .filter((model) => model.modelId.trim().length > 0)
    .map((model) => ({
      label: model.displayName || model.modelId,
      modelCode: model.modelId,
      optionCode: systemAutomationModelOptionCode(agent.agentCode, model.modelId),
    }));
}

export function automationModelCodeFromOption(
  options: AutomationModelOption[],
  optionCode: string,
) {
  return options.find((option) => option.optionCode === optionCode)?.modelCode ?? null;
}

export function optionCodeForAutomationModel(agentCode: string, modelCode: string | null) {
  if (!modelCode) return isMainAutomationAgent(agentCode) ? mainDefaultModelOptionCode : "";
  if (isMainAutomationAgent(agentCode)) return mainAutomationModelOptionCode(modelCode);
  return systemAutomationModelOptionCode(agentCode, modelCode);
}

function mainAutomationModelOptionCode(modelCode: string) {
  return `main:${modelCode}`;
}

function systemAutomationModelOptionCode(agentCode: string, modelId: string) {
  return `system:${agentCode}:${modelId}`;
}

function isAutomationTextModelOption(model: ModelSelectOption) {
  return model.enabled && model.modelType === "TEXT";
}
