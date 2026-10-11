import { requestApi } from "@/shared/api/httpClient";

import type {
  InstallRegistrySkillPayload,
  RegistrySkillDetail,
  RegistrySkillMutationResult,
  RegistrySkillRecord,
  RegistrySkillSyncResult,
  RegistrySkillUninstallResult,
  SkillSearchCandidate,
  SkillSearchMode,
} from "@/domain/skill/model";

export const skillApiEndpoints = {
  searchMarketplaceSkills: "POST /api/v1/skills/search",
  listRegistrySkills: "GET /api/v1/skills/registry",
  getRegistrySkill: "GET /api/v1/skills/registry/{skillName}",
  uninstallRegistrySkill: "DELETE /api/v1/skills/registry/{skillName}",
  installRegistrySkill: "POST /api/v1/skills/registry/install",
  rescanRegistrySkills: "POST /api/v1/skills/registry/rescan",
  checkRegistrySkillUpdates: "POST /api/v1/skills/registry/check-updates",
  updateRegistrySkills: "POST /api/v1/skills/registry/update",
} as const;

export async function searchMarketplaceSkills(
  query: string,
  mode: SkillSearchMode
) {
  return requestApi<SkillSearchCandidate[]>(
    skillApiEndpoints.searchMarketplaceSkills,
    {
      body: { mode, query },
    }
  );
}

export async function fetchRegistrySkills(
  options: {
    search?: string;
  } = {}
) {
  return requestApi<RegistrySkillRecord[]>(
    skillApiEndpoints.listRegistrySkills,
    {
      queryParams: {
        search: options.search,
      },
    }
  );
}

export async function fetchRegistrySkill(skillName: string) {
  return requestApi<RegistrySkillDetail>(skillApiEndpoints.getRegistrySkill, {
    pathParams: { skillName },
  });
}

export async function uninstallRegistrySkill(skillName: string) {
  return requestApi<RegistrySkillUninstallResult>(
    skillApiEndpoints.uninstallRegistrySkill,
    {
      pathParams: { skillName },
    }
  );
}

export async function installRegistrySkill(
  payload: InstallRegistrySkillPayload
) {
  return requestApi<RegistrySkillMutationResult>(
    skillApiEndpoints.installRegistrySkill,
    {
      body: payload,
    }
  );
}

export async function rescanRegistrySkills() {
  return requestApi<RegistrySkillSyncResult>(
    skillApiEndpoints.rescanRegistrySkills,
    {
      body: {},
    }
  );
}

export async function checkRegistrySkillUpdates() {
  return requestApi<RegistrySkillSyncResult>(
    skillApiEndpoints.checkRegistrySkillUpdates,
    {
      body: {},
    }
  );
}

export async function updateRegistrySkills() {
  return requestApi<RegistrySkillSyncResult>(
    skillApiEndpoints.updateRegistrySkills,
    {
      body: {},
    }
  );
}

export function fetchAgentSkills(agentCode: string) {
  return requestApi<RegistrySkillRecord[]>(
    "GET /api/v1/agents/registry/internal/{agentCode}/skills",
    { pathParams: { agentCode } }
  );
}
export function setAgentSkills(agentCode: string, skillNames: string[]) {
  return requestApi<RegistrySkillRecord[]>(
    "PUT /api/v1/agents/registry/internal/{agentCode}/skills",
    { pathParams: { agentCode }, body: { skillNames } }
  );
}
export function fetchSkillAgents(skillName: string) {
  return requestApi<{ agentCodes: string[] }>(
    "GET /api/v1/skills/registry/{skillName}/agent-grants",
    { pathParams: { skillName } }
  );
}
export function setSkillAgents(skillName: string, agentCodes: string[]) {
  return requestApi<{ agentCodes: string[] }>(
    "PUT /api/v1/skills/registry/{skillName}/agent-grants",
    { pathParams: { skillName }, body: { agentCodes } }
  );
}
export const CAPABILITY_RESOURCES_CHANGED = "tron:capability-resources-changed";
export function notifyCapabilityResourcesChanged() {
  window.dispatchEvent(new Event(CAPABILITY_RESOURCES_CHANGED));
}
