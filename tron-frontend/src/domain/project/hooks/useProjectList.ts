import { useCallback, useEffect, useState } from "react";
import { fetchProjects, saveProject } from "@/domain/project/api/projectApi";
import { listMockProjects, saveMockProject } from "@/domain/project/mock/data";
import type {
  ProjectRecord,
  SaveProjectInput,
} from "@/domain/project/model/ProjectModel";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function useProjectList(workspaceCode: string) {
  const [refreshKey, setRefreshKey] = useState(0);
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    if (!workspaceCode) {
      setProjects([]);
      return;
    }
    if (getServiceDataSource("project") === "mock") {
      setProjects(listMockProjects(workspaceCode));
      return;
    }
    const controller = new AbortController();
    setLoading(true);
    fetchProjects(workspaceCode)
      .then(setProjects)
      .catch(() => setProjects([]))
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [workspaceCode, refreshKey]);
  return {
    projects,
    loading,
    refresh: useCallback(() => setRefreshKey((value) => value + 1), []),
  };
}

export function createProject(input: SaveProjectInput): Promise<ProjectRecord> {
  return getServiceDataSource("project") === "mock"
    ? Promise.resolve(saveMockProject(input))
    : saveProject(input);
}
