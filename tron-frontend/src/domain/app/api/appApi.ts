import { requestApi } from "@/shared/api/httpClient";

import type {
  AppDetailData,
  AppLifecycleResult,
  AppListItem,
  AppListQuery,
  AppOption,
  AppOptionQuery,
  AppPage,
  CreateAppPayload,
  UpdateAppPayload,
} from "@/domain/app/model";

export const appApiEndpoints = {
  archiveApp: "POST /api/v1/apps/{appCode}/archive",
  createApp: "POST /api/v1/apps/workspaces/{workspaceCode}",
  getAppDetail: "GET /api/v1/apps/{appCode}",
  listAppOptions: "GET /api/v1/apps/workspaces/{workspaceCode}/options",
  listApps: "GET /api/v1/apps/workspaces/{workspaceCode}",
  restoreApp: "POST /api/v1/apps/{appCode}/restore",
  updateApp: "PATCH /api/v1/apps/{appCode}",
} as const;

type AppEndpointParams = Record<string, string | number | boolean | null | undefined>;

function appListQueryParams(query: AppListQuery): AppEndpointParams {
  return {
    current: query.current,
    includeArchived: query.includeArchived,
    runtimeRole: query.runtimeRole,
    search: query.search,
    size: query.size,
    status: query.status,
  };
}

function appOptionQueryParams(query: AppOptionQuery): AppEndpointParams {
  return {
    runtimeRole: query.runtimeRole,
    search: query.search,
  };
}

export function fetchAppPage(query: AppListQuery, signal?: AbortSignal) {
  return requestApi<AppPage<AppListItem>>(appApiEndpoints.listApps, {
    pathParams: { workspaceCode: query.workspaceCode },
    queryParams: appListQueryParams(query),
    signal,
  });
}

export function fetchAppOptions(query: AppOptionQuery, signal?: AbortSignal) {
  return requestApi<AppOption[]>(appApiEndpoints.listAppOptions, {
    pathParams: { workspaceCode: query.workspaceCode },
    queryParams: appOptionQueryParams(query),
    signal,
  });
}

export function fetchAppDetail(appCode: string, signal?: AbortSignal) {
  return requestApi<AppDetailData>(appApiEndpoints.getAppDetail, {
    pathParams: { appCode },
    signal,
  });
}

export function createApp(workspaceCode: string, payload: CreateAppPayload) {
  return requestApi<AppDetailData>(appApiEndpoints.createApp, {
    body: payload,
    pathParams: { workspaceCode },
  });
}

export function updateApp(appCode: string, payload: UpdateAppPayload) {
  return requestApi<AppDetailData>(appApiEndpoints.updateApp, {
    body: payload,
    pathParams: { appCode },
  });
}

export function archiveApp(appCode: string) {
  return requestApi<AppLifecycleResult>(appApiEndpoints.archiveApp, {
    pathParams: { appCode },
  });
}

export function restoreApp(appCode: string) {
  return requestApi<AppLifecycleResult>(appApiEndpoints.restoreApp, {
    pathParams: { appCode },
  });
}

export function deleteApp(appCode: string) {
  return requestApi<{ appCode: string }>("DELETE /api/v1/apps/{appCode}", { pathParams: { appCode } });
}
