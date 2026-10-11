import type { TFunction } from "i18next";

import type {
  WorkspaceInternalAgentRecord,
  WorkspaceSystemAgentRecord,
} from "@/domain/agent/model";

const internalAgentCatalogKeyByCode: Record<string, string> = {
  "main-agent": "mainAgent",
  explore: "explore",
  "full-stack": "fullStack",
  product: "product",
};

const defaultInternalAgentNameByCode: Record<string, string> = {
  "main-agent": "Main Agent",
  explore: "探索智能体",
  "full-stack": "全栈工程师",
  product: "杨布斯",
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
  const key = internalAgentCatalogKeyByCode[agent.agentCode];
  return key
    ? t(`pages:workspaceAgents.internal.catalog.${key}.description`, { defaultValue: agent.description })
    : agent.description;
}

export function translateSystemAgentDescription(t: TFunction, agent: Pick<WorkspaceSystemAgentRecord, "agentCode" | "description">) {
  const key = systemAgentCatalogKeyByCode[agent.agentCode];
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
