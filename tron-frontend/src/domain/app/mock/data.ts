import type {
  AppDetailData,
  AppLifecycleResult,
  AppListItem,
  AppListQuery,
  AppOption,
  AppOptionQuery,
  AppPage,
  AppRuntimeRole,
  CreateAppPayload,
  UpdateAppPayload,
} from "@/domain/app/model";

const NOW = "2026-05-19T12:00:00Z";

let nextAppId = 734_900_000_000_000_004n;

let apps: AppDetailData[] = [
  {
    id: "734900000000000001",
    appCode: "APP-TRON-CONSOLE",
    workspaceCode: "TRON_CORE",
    name: "Tron Console",
    description: "Primary operator-facing frontend.",
    localDirectory: "/workspace/tron/tron-frontend",
    runtimeRole: "Frontend",
    repositoryUrl: "https://github.com/clouds3n/tron-console",
    defaultBranch: "master",
    tags: ["Operator", "React"],
    status: "Active",
    createdAt: NOW,
    updatedAt: NOW,
    archivedAt: null,
  },
  {
    id: "734900000000000002",
    appCode: "APP-TRON-API",
    workspaceCode: "TRON_CORE",
    name: "Tron API",
    description: "Core backend serving task and admin workflows.",
    localDirectory: "/workspace/tron/tron-backend",
    runtimeRole: "Backend",
    repositoryUrl: "https://github.com/clouds3n/tron-backend",
    defaultBranch: "master",
    tags: ["Core", "Rust"],
    status: "Active",
    createdAt: NOW,
    updatedAt: NOW,
    archivedAt: null,
  },
  {
    id: "734900000000000003",
    appCode: "APP-COMPLIANCE-API",
    workspaceCode: "COMPLIANCE",
    name: "Compliance API",
    description: "Backend used by regulated reporting workflows.",
    localDirectory: "/workspace/apps/compliance-api",
    runtimeRole: "Backend",
    repositoryUrl: "https://github.com/clouds3n/compliance-api",
    defaultBranch: "master",
    tags: ["Audit"],
    status: "Active",
    createdAt: NOW,
    updatedAt: NOW,
    archivedAt: null,
  },
  {
    id: "734900000000000004",
    appCode: "APP-TRON-LEGACY",
    workspaceCode: "TRON_CORE",
    name: "Legacy Console",
    description: "Archived frontend kept for historical reference.",
    localDirectory: "/workspace/tron/legacy-console",
    runtimeRole: "Frontend",
    repositoryUrl: null,
    defaultBranch: null,
    tags: ["Legacy"],
    status: "Archived",
    createdAt: NOW,
    updatedAt: NOW,
    archivedAt: NOW,
  },
];

function normalizeText(value?: string | null) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

function normalizeTags(tags: string[]) {
  return Array.from(new Set(tags.map((tag) => tag.trim()).filter(Boolean))).sort();
}

function matchesSearch(app: AppDetailData, search?: string) {
  if (!search?.trim()) return true;
  const value = [
    app.appCode,
    app.name,
    app.description ?? "",
    app.localDirectory,
    app.repositoryUrl ?? "",
    app.defaultBranch ?? "",
    app.runtimeRole,
    app.tags.join(" "),
  ].join(" ").toLowerCase();
  return value.includes(search.trim().toLowerCase());
}

function validateUniqueName(workspaceCode: string, name: string, currentAppCode?: string) {
  if (apps.some((app) => app.workspaceCode === workspaceCode && app.appCode !== currentAppCode && app.name.toLowerCase() === name.toLowerCase())) {
    throw new Error("App name already exists in the selected workspace");
  }
}

function requireApp(appCode: string) {
  const app = apps.find((item) => item.appCode === appCode);
  if (!app) throw new Error(`App ${appCode} was not found.`);
  return app;
}

function toListItem(app: AppDetailData): AppListItem {
  return {
    appCode: app.appCode,
    workspaceCode: app.workspaceCode,
    name: app.name,
    description: app.description,
    localDirectory: app.localDirectory,
    runtimeRole: app.runtimeRole,
    repositoryUrl: app.repositoryUrl,
    defaultBranch: app.defaultBranch,
    tags: [...app.tags],
    status: app.status,
    updatedAt: app.updatedAt,
    archivedAt: app.archivedAt,
  };
}

function lifecycleResult(app: AppDetailData): AppLifecycleResult {
  return {
    appCode: app.appCode,
    status: app.status,
    archivedAt: app.archivedAt,
  };
}

export function listAppPage(query: AppListQuery): AppPage<AppListItem> {
  const current = Math.max(1, query.current ?? 1);
  const size = Math.max(1, query.size ?? 15);
  const filtered = apps
    .filter((app) => !query.workspaceCode || app.workspaceCode === query.workspaceCode)
    .filter((app) => query.runtimeRole ? app.runtimeRole === query.runtimeRole : true)
    .filter((app) => {
      if (query.status) return app.status === query.status;
      if (query.includeArchived) return true;
      return app.status === "Active";
    })
    .filter((app) => matchesSearch(app, query.search))
    .sort((left, right) => left.name.localeCompare(right.name) || left.appCode.localeCompare(right.appCode));
  const start = (current - 1) * size;

  return {
    current,
    size,
    total: filtered.length,
    records: filtered.slice(start, start + size).map(toListItem),
  };
}

export function listAppOptions(query: AppOptionQuery): AppOption[] {
  return apps
    .filter((app) => app.workspaceCode === query.workspaceCode)
    .filter((app) => app.status === "Active")
    .filter((app) => query.runtimeRole ? app.runtimeRole === query.runtimeRole : true)
    .filter((app) => matchesSearch(app, query.search))
    .sort((left, right) => left.name.localeCompare(right.name) || left.appCode.localeCompare(right.appCode))
    .map((app) => ({
      appCode: app.appCode,
      name: app.name,
      description: app.description,
      runtimeRole: app.runtimeRole,
    }));
}

export function getAppDetail(appCode: string): AppDetailData {
  const app = requireApp(appCode);
  return {
    ...app,
    tags: [...app.tags],
  };
}

export function createMockApp(workspaceCode: string, payload: CreateAppPayload): AppDetailData {
  const name = payload.name.trim();
  const localDirectory = payload.localDirectory.trim();
  if (!workspaceCode.trim()) throw new Error("Workspace selection is required");
  if (!name) throw new Error("App name is required");
  if (!localDirectory) throw new Error("Local directory is required");
  validateUniqueName(workspaceCode, name);
  nextAppId += 1n;
  const app: AppDetailData = {
    id: nextAppId.toString(),
    appCode: `APP-${nextAppId.toString()}`,
    workspaceCode,
    name,
    description: normalizeText(payload.description),
    localDirectory,
    runtimeRole: payload.runtimeRole,
    repositoryUrl: normalizeText(payload.repositoryUrl),
    defaultBranch: normalizeText(payload.defaultBranch),
    tags: normalizeTags(payload.tags),
    status: "Active",
    createdAt: NOW,
    updatedAt: NOW,
    archivedAt: null,
  };
  apps = [app, ...apps];
  return getAppDetail(app.appCode);
}

export function updateMockApp(appCode: string, payload: UpdateAppPayload): AppDetailData {
  const current = requireApp(appCode);
  if (current.status === "Archived") throw new Error("Archived app cannot be modified");
  const name = payload.name.trim();
  const localDirectory = payload.localDirectory.trim();
  if (!name) throw new Error("App name is required");
  if (!localDirectory) throw new Error("Local directory is required");
  validateUniqueName(current.workspaceCode, name, current.appCode);

  apps = apps.map((app) => app.appCode !== appCode ? app : {
    ...app,
    name,
    description: normalizeText(payload.description),
    localDirectory,
    runtimeRole: payload.runtimeRole,
    repositoryUrl: normalizeText(payload.repositoryUrl),
    defaultBranch: normalizeText(payload.defaultBranch),
    tags: normalizeTags(payload.tags),
    updatedAt: NOW,
  });

  return getAppDetail(appCode);
}

export function archiveMockApp(appCode: string): AppLifecycleResult {
  requireApp(appCode);
  apps = apps.map((app) => app.appCode !== appCode ? app : {
    ...app,
    status: "Archived",
    archivedAt: NOW,
    updatedAt: NOW,
  });
  return lifecycleResult(requireApp(appCode));
}

export function restoreMockApp(appCode: string): AppLifecycleResult {
  requireApp(appCode);
  apps = apps.map((app) => app.appCode !== appCode ? app : {
    ...app,
    status: "Active",
    archivedAt: null,
    updatedAt: NOW,
  });
  return lifecycleResult(requireApp(appCode));
}

export function isKnownAppCode(workspaceCode: string, appCode: string, runtimeRole?: AppRuntimeRole) {
  return listAppOptions({ workspaceCode, runtimeRole }).some((app) => app.appCode === appCode);
}
