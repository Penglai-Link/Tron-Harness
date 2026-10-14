import type { TFunction } from "i18next";

import type {
  WorkspaceInternalAgentRecord,
  WorkspaceSystemAgentRecord,
} from "@/domain/agent/model";

const internalAgentCatalogKeyByCode: Record<string, string> = {
  "main-agent": "mainAgent",
  "explore-agent": "explore",
  "fullstack-agent": "fullStack",
  "prd-agent": "product",
};

const defaultInternalAgentNameByCode: Record<string, string> = {
  "main-agent": "Main Agent",
  "explore-agent": "探索智能体",
  "fullstack-agent": "全栈工程师",
  "prd-agent": "杨布斯",
};

const internalAgentGroupCatalogKeyByName: Record<string, string> = {
  "主智能体": "mainAgent",
  "探索": "explore",
  "开发": "development",
  "设计": "design",
};

const systemAgentCatalogKeyByCode: Record<string, string> = {
  codex: "codex",
  opencode: "opencode",
};

export function translateInternalAgentName(t: TFunction, agent: WorkspaceInternalAgentRecord) {
  const key = agent.name === defaultInternalAgentNameByCode[agent.agentCode] || agent.name === "Main Agent"
    ? internalAgentCatalogKeyByCode[agent.agentCode]
    : undefined;
  return key
    ? t(`pages:workspaceAgents.internal.catalog.${key}.name`, { defaultValue: agent.name })
    : agent.name;
}

export function translateInternalAgentGroupName(t: TFunction, name: string) {
  const key = internalAgentGroupCatalogKeyByName[name];
  return key
    ? t(`pages:workspaceAgents.internal.groups.catalog.${key}`, { defaultValue: name })
    : name;
}

export function translateInternalAgentDescription(t: TFunction, agent: WorkspaceInternalAgentRecord) {
  const defaults: Record<string, string[]> = {
    "main-agent": ["Primary workspace internal agent with access to platform business tools.", "Primary platform internal agent used for workspace chat runtime policy."],
    "explore-agent": ["只读探索任务使用的内置智能体。"],
    "fullstack-agent": ["用于全栈开发任务的内置智能体。"],
    "prd-agent": ["用于产品分析与设计任务的内置智能体。"],
  };
  const key = defaults[agent.agentCode]?.includes(agent.description) ? internalAgentCatalogKeyByCode[agent.agentCode] : undefined;
  return key
    ? t(`pages:workspaceAgents.internal.catalog.${key}.description`, { defaultValue: agent.description })
    : agent.description;
}

export function translateSystemAgentDescription(t: TFunction, agent: Pick<WorkspaceSystemAgentRecord, "agentCode" | "baseAgentCode" | "description">) {
  const defaults: Record<string, string[]> = {
    codex: ["OpenAI Codex CLI agent runtime.", "OpenAI coding agent runtime used for structured feature delivery."],
    opencode: ["OpenCode CLI agent runtime.", "Local CLI coding agent kept as a platform-level fallback executor."],
  };
  const baseCode = agent.baseAgentCode || agent.agentCode;
  const key = defaults[baseCode]?.includes(agent.description) ? systemAgentCatalogKeyByCode[baseCode] : undefined;
  return key
    ? t(`pages:workspaceAgents.table.catalog.${key}.description`, { defaultValue: agent.description })
    : agent.description;
}

export function translateSystemAgentSource(
  t: TFunction,
  source: WorkspaceSystemAgentRecord["source"],
) {
  if (source === "System Scan") {
    return t("pages:workspaceAgents.table.values.systemScan", { defaultValue: source });
  }
  return source;
}

export function translateInheritedFrom(
  t: TFunction,
  inheritedFrom: WorkspaceSystemAgentRecord["inheritedFrom"],
) {
  if (inheritedFrom === "Platform") {
    return t("pages:workspaceAgents.table.values.inheritedPlatform", { defaultValue: inheritedFrom });
  }
  return inheritedFrom;
}
