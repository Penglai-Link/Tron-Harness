import type {
  ProjectRecord,
  SaveProjectInput,
} from "@/domain/project/model/ProjectModel";

let projects: ProjectRecord[] = [
  {
    projectCode: "PROJECT-TRON",
    workspaceCode: "TRON_CORE",
    name: "Tron",
    appCodes: ["APP-TRON-CONSOLE", "APP-TRON-API"],
    primaryAppCode: "APP-TRON-CONSOLE",
    updatedAt: "2026-09-14T08:00:00Z",
  },
];

export function listMockProjects(workspaceCode: string) {
  return projects.filter((project) => project.workspaceCode === workspaceCode);
}

export function saveMockProject(input: SaveProjectInput): ProjectRecord {
  const projectCode = input.projectCode ?? `PROJECT-${Date.now()}`;
  const record: ProjectRecord = {
    projectCode,
    workspaceCode: input.workspaceCode,
    name: input.name.trim(),
    appCodes: input.appCodes,
    primaryAppCode: input.primaryAppCode ?? input.appCodes[0] ?? null,
    updatedAt: new Date().toISOString(),
  };
  projects = [
    record,
    ...projects.filter((project) => project.projectCode !== projectCode),
  ];
  return record;
}

export function archiveMockProject(projectCode: string) {
  projects = projects.filter((project) => project.projectCode !== projectCode);
}
