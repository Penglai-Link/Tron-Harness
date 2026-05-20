import { requestApi } from "@/shared/api/httpClient";

import type {
  InstallRegistrySkillPayload,
  RegistrySkillDetail,
  RegistrySkillMutationResult,
  RegistrySkillRecord,
  RegistrySkillSyncResult,
  RegistrySkillUninstallResult,
  SkillBindingMutationResult,
  SkillSearchCandidate,
  SkillSearchMode,
  WorkspaceSkillDetail,
  WorkspaceSkillRecord,
} from "@/domain/skill/model";

export const skillApiEndpoints = {
  listCapabilities: "GET /api/v1/capabilities",
  getSkillSchemaPreview: "GET /api/v1/skills/schema-preview",
  searchMarketplaceSkills: "POST /api/v1/skills/search",
  listRegistrySkills: "GET /api/v1/skills/registry",
  getRegistrySkill: "GET /api/v1/skills/registry/{skillName}",
  uninstallRegistrySkill: "DELETE /api/v1/skills/registry/{skillName}",
  listWorkspaceSkills: "GET /api/v1/skills/workspaces/{workspaceCode}/skills",
  getWorkspaceSkill: "GET /api/v1/skills/workspaces/{workspaceCode}/skills/{skillName}",
  installRegistrySkill: "POST /api/v1/skills/registry/install",
  rescanRegistrySkills: "POST /api/v1/skills/registry/rescan",
  checkRegistrySkillUpdates: "POST /api/v1/skills/registry/check-updates",
  updateRegistrySkills: "POST /api/v1/skills/registry/update",
  refreshWorkspaceSkills: "POST /api/v1/skills/workspaces/{workspaceCode}/refresh",
  bindWorkspaceSkill: "POST /api/v1/skills/workspaces/{workspaceCode}/bindings",
  unbindWorkspaceSkill: "DELETE /api/v1/skills/workspaces/{workspaceCode}/bindings/{skillName}",
} as const;

export async function searchMarketplaceSkills(query: string, mode: SkillSearchMode) {
  return requestApi<SkillSearchCandidate[]>(skillApiEndpoints.searchMarketplaceSkills, {
    body: { mode, query },
  });
}

export async function fetchRegistrySkills(options: {
  workspaceCode?: string;
  search?: string;
} = {}) {
  return requestApi<RegistrySkillRecord[]>(skillApiEndpoints.listRegistrySkills, {
    queryParams: {
      search: options.search,
      workspaceCode: options.workspaceCode,
    },
  });
}

export async function fetchRegistrySkill(skillName: string) {
  return requestApi<RegistrySkillDetail>(skillApiEndpoints.getRegistrySkill, {
    pathParams: { skillName },
  });
}

export async function uninstallRegistrySkill(skillName: string, removeBindings = false) {
  return requestApi<RegistrySkillUninstallResult>(skillApiEndpoints.uninstallRegistrySkill, {
    pathParams: { skillName },
    queryParams: { removeBindings },
  });
}

export async function fetchRegistrySkillForWorkspace(skillName: string, workspaceCode?: string) {
  return requestApi<RegistrySkillDetail>(skillApiEndpoints.getRegistrySkill, {
    pathParams: { skillName },
    queryParams: { workspaceCode },
  });
}

export async function fetchWorkspaceSkills(workspaceCode: string, options: {
  search?: string;
} = {}) {
  return requestApi<WorkspaceSkillRecord[]>(skillApiEndpoints.listWorkspaceSkills, {
    pathParams: { workspaceCode },
    queryParams: {
      search: options.search,
    },
  });
}

export async function fetchWorkspaceSkill(workspaceCode: string, skillName: string) {
  return requestApi<WorkspaceSkillDetail>(skillApiEndpoints.getWorkspaceSkill, {
    pathParams: { workspaceCode, skillName },
  });
}

export async function installRegistrySkill(payload: InstallRegistrySkillPayload) {
  return requestApi<RegistrySkillMutationResult>(skillApiEndpoints.installRegistrySkill, {
    body: payload,
  });
}

export async function rescanRegistrySkills() {
  return requestApi<RegistrySkillSyncResult>(skillApiEndpoints.rescanRegistrySkills, {
    body: {},
  });
}

export async function checkRegistrySkillUpdates() {
  return requestApi<RegistrySkillSyncResult>(skillApiEndpoints.checkRegistrySkillUpdates, {
    body: {},
  });
}

export async function updateRegistrySkills() {
  return requestApi<RegistrySkillSyncResult>(skillApiEndpoints.updateRegistrySkills, {
    body: {},
  });
}

export async function refreshWorkspaceSkills(workspaceCode: string) {
  return requestApi<RegistrySkillSyncResult>(skillApiEndpoints.refreshWorkspaceSkills, {
    body: {},
    pathParams: { workspaceCode },
  });
}

export async function bindWorkspaceSkill(workspaceCode: string, skillName: string, enabled = true) {
  return requestApi<SkillBindingMutationResult>(skillApiEndpoints.bindWorkspaceSkill, {
    body: { skillName, enabled },
    pathParams: { workspaceCode },
  });
}

export async function unbindWorkspaceSkill(workspaceCode: string, skillName: string) {
  return requestApi<SkillBindingMutationResult>(skillApiEndpoints.unbindWorkspaceSkill, {
    pathParams: { workspaceCode, skillName },
  });
}
