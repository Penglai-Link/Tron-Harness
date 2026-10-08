import type { TFunction } from "i18next";

import type {
  WorkspaceInternalAgentRecord,
  WorkspaceSystemAgentRecord,
} from "@/domain/agent/model";

const internalAgentCatalogKeyByCode: Record<string, string> = {
  "main-agent": "mainAgent",
};

const systemAgentCatalogKeyByCode: Record<string, string> = {
  codex: "codex",
  opencode: "opencode",
};

export function translateInternalAgentName(t: TFunction, agent: WorkspaceInternalAgentRecord) {
  const key = agent.name === "Main Agent" ? internalAgentCatalogKeyByCode[agent.agentCode] : undefined;
  return key
    ? t(`pages:workspaceAgents.internal.catalog.${key}.name`, { defaultValue: agent.name })
    : agent.name;
}

export function translateInternalAgentDescription(t: TFunction, agent: WorkspaceInternalAgentRecord) {
  const key = internalAgentCatalogKeyByCode[agent.agentCode];
  return key
    ? t(`pages:workspaceAgents.internal.catalog.${key}.description`, { defaultValue: agent.description })
    : agent.description;
}

export function translateSystemAgentDescription(t: TFunction, agent: WorkspaceSystemAgentRecord) {
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
