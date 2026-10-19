import type {
  SystemAgentModelRecord,
  WorkspaceInternalAgentRecord,
  WorkspaceSystemAgentRecord,
} from "@/domain/agent/model";
import type {
  ModelSelection,
  ModelSelectOption,
  ProviderModelQuality,
} from "@/domain/model/model/ModelRegistryModel";

export interface AutomationAgentOption {
  agentCode: string;
  description: string;
  models: SystemAgentModelRecord[];
  modelOptionSource?: "workspace" | "agent";
  name: string;
}

export interface AutomationModelOption {
  accessPlatform?: string | null;
  automatic?: boolean;
  displayName?: string;
  label: string;
  modelCode: string | null;
  modelName?: string;
  modelSelection?: ModelSelection | null;
  optionCode: string;
  outputCoefficient?: number | null;
  providerCode?: string | null;
  qualityType?: ProviderModelQuality | null;
  upstreamVendorName?: string | null;
  vendor?: string | null;
}

export const mainAutomationAgentOption: AutomationAgentOption = {
  agentCode: "main-agent",
  description: "Default workspace chat agent.",
  models: [],
  modelOptionSource: "workspace",
  name: "Main Agent",
};

const mainDefaultModelOptionCode = "main:default";
const workspaceDefaultModelOptionPrefix = "workspace";

export function isMainAutomationAgent(agentCode: string) {
  return !agentCode.trim() || agentCode.trim() === mainAutomationAgentOption.agentCode;
}

export function availableAutomationAgents(
  systemAgents: WorkspaceSystemAgentRecord[],
  internalAgents: WorkspaceInternalAgentRecord[] = [],
): AutomationAgentOption[] {
  const availableInternalAgents = internalAgents
    .filter((agent) => agent.enabled !== false)
    .map((agent) => ({
      agentCode: agent.agentCode,
      description: agent.description,
      models: [],
      modelOptionSource: "workspace" as const,
      name: agent.name,
    }));
  const availableSystemAgents = systemAgents
    .filter((agent) => agent.enabled && agent.available && agent.status === "Enabled")
    .map((agent) => ({
      agentCode: agent.agentCode,
      description: agent.description,
      models: agent.models,
      modelOptionSource: "agent" as const,
      name: agent.name,
    }));

  return [...availableInternalAgents, ...availableSystemAgents];
}

export function automationModelOptionsForAgent(
  workspaceModels: ModelSelectOption[],
  agent: AutomationAgentOption,
  automaticModelLabel = "Conversation Automatic Routing",
): AutomationModelOption[] {
  if (agent.modelOptionSource === "workspace" || isMainAutomationAgent(agent.agentCode)) {
    return [{
      automatic: true,
      label: automaticModelLabel,
      modelCode: null,
      modelName: "",
      modelSelection: {
        kind: "Auto",
        providerCode: null,
        modelCode: null,
        qualityType: null,
        routeCode: null,
      },
      optionCode: workspaceDefaultModelOptionCode(agent.agentCode),
    }, ...workspaceModels
      .filter((model) => isAutomationTextModelOption(model))
      .map((model) => ({
        label: `${model.displayName || model.modelName} (${model.modelName})`,
        displayName: model.displayName || model.modelName,
        modelCode: model.modelCode,
        modelName: model.modelName,
        modelSelection: model.modelSelection ?? null,
        optionCode: workspaceAutomationModelOptionCode(agent.agentCode, model.modelCode),
        accessPlatform: model.accessPlatform ?? null,
        providerCode: model.providerCode ?? null,
        qualityType: model.qualityType ?? model.modelSelection?.qualityType ?? null,
        upstreamVendorName: model.upstreamVendorName ?? null,
        vendor: model.vendor ?? null,
        outputCoefficient: model.outputCoefficient ?? null,
      }))];
  }

  return agent.models
    .filter((model) => model.modelId.trim().length > 0)
    .map((model) => ({
      label: model.displayName || model.modelId,
      displayName: model.displayName || model.modelId,
      modelCode: model.modelId,
      modelName: model.modelId,
      optionCode: systemAutomationModelOptionCode(agent.agentCode, model.modelId),
    }));
}

export function automationModelCodeFromOption(
  options: AutomationModelOption[],
  optionCode: string,
) {
  return options.find((option) => option.optionCode === optionCode)?.modelCode ?? null;
}

export function optionCodeForAutomationModel(
  agentCode: string,
  modelCode: string | null,
  modelOptionSource?: AutomationAgentOption["modelOptionSource"],
) {
  const usesWorkspaceModels = modelOptionSource === "workspace" || isMainAutomationAgent(agentCode);
  if (!modelCode) return usesWorkspaceModels ? workspaceDefaultModelOptionCode(agentCode) : "";
  if (usesWorkspaceModels) return workspaceAutomationModelOptionCode(agentCode, modelCode);
  return systemAutomationModelOptionCode(agentCode, modelCode);
}

function mainAutomationModelOptionCode(modelCode: string) {
  return `main:${modelCode}`;
}

function workspaceDefaultModelOptionCode(agentCode: string) {
  return isMainAutomationAgent(agentCode)
    ? mainDefaultModelOptionCode
    : `${workspaceDefaultModelOptionPrefix}:${agentCode}:default`;
}

function workspaceAutomationModelOptionCode(agentCode: string, modelCode: string) {
  return isMainAutomationAgent(agentCode)
    ? mainAutomationModelOptionCode(modelCode)
    : `${workspaceDefaultModelOptionPrefix}:${agentCode}:${modelCode}`;
}

function systemAutomationModelOptionCode(agentCode: string, modelId: string) {
  return `system:${agentCode}:${modelId}`;
}

function isAutomationTextModelOption(model: ModelSelectOption) {
  return model.enabled && model.modelType === "TEXT";
}
