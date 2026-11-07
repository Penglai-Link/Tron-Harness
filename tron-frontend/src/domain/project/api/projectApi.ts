import { requestApi } from "@/shared/api/httpClient";
import type {
  ProjectRecord,
  SaveProjectInput,
} from "@/domain/project/model/ProjectModel";

export const projectApiEndpoints = {
  list: "POST /api/v1/projects/list",
  get: "POST /api/v1/projects/get",
  save: "POST /api/v1/projects/save",
  archive: "POST /api/v1/projects/archive",
  delete: "POST /api/v1/projects/delete",
} as const;

export function getProject(projectCode: string, signal?: AbortSignal) {
  return requestApi<ProjectRecord>(projectApiEndpoints.get, { body: { projectCode }, signal });
}

export function fetchProjects(workspaceCode: string) {
  return requestApi<ProjectRecord[]>(projectApiEndpoints.list, {
    body: { workspaceCode },
  });
}

export function saveProject(input: SaveProjectInput) {
  return requestApi<ProjectRecord>(projectApiEndpoints.save, { body: input });
}

export function archiveProject(projectCode: string) {
  return requestApi<{ sessionCodes: string[] }>(projectApiEndpoints.archive, {
    body: { projectCode },
  });
}

export function deleteProject(projectCode: string) {
  return requestApi<{ sessionCodes: string[] }>(projectApiEndpoints.delete, { body: { projectCode } });
}
