import { isApiError, requestApi } from "@/shared/api/httpClient";

export type CenterStatus =
  | "all"
  | "installed"
  | "updatable"
  | "enabled"
  | "disabled";
export interface CenterQuery {
  current: number;
  size: number;
  keyword: string;
  category: string;
  source: string;
  status: CenterStatus;
}
export interface CenterPack {
  packId: string;
  releaseId: string | null;
  name: string;
  description: string;
  author: string;
  version: string;
  source: string;
  category: string | null;
  installedVersion: string | null;
  enabled: boolean;
  builtin: boolean;
  updateAvailable: boolean;
  updateCheckFailed: boolean;
  fileSize: number | null;
  downloads: number | null;
  likeCount: number | null;
  liked: boolean | null;
}
export interface CenterPage {
  current: number;
  size: number;
  total: number;
  records: CenterPack[];
  updateCheckIncomplete: boolean;
}
export interface CenterDetail {
  pack: CenterPack;
  manifest: {
    framework: string;
    permissions?: string[];
    dependencies?: { id: string; version: string }[];
  };
  readme: string | null;
  releaseNotes: string | null;
  signingKeyId: string | null;
  installable: boolean;
  installBlockReason: string | null;
}
export interface CenterVersions {
  current: number;
  size: number;
  total: number;
  records: { releaseId: string; version: string }[];
}
export function isCenterReleaseUnavailable(error: unknown) {
  return isApiError(error) && error.code === "101038004";
}
export function getCenterPage(query: CenterQuery, signal?: AbortSignal) {
  return requestApi<CenterPage>("GET /api/v1/capabilities/center", {
    signal,
    queryParams: {
      ...query,
      keyword: query.keyword || undefined,
      category: query.category || undefined,
      source: query.source || undefined,
    },
  });
}
export function getCenterCategories(signal?: AbortSignal) {
  return requestApi<{ categories: string[] }>(
    "GET /api/v1/capabilities/center/categories",
    { signal }
  );
}
export function getCenterDetail(id: string, signal?: AbortSignal) {
  return requestApi<CenterDetail>(
    "GET /api/v1/capabilities/center/releases/{id}",
    { pathParams: { id }, signal }
  );
}
export function getCenterVersions(
  id: string,
  current: number,
  signal?: AbortSignal
) {
  return requestApi<CenterVersions>(
    "GET /api/v1/capabilities/center/packs/{id}/versions",
    { pathParams: { id }, queryParams: { current, size: 20 }, signal }
  );
}
export async function installCenterRelease(
  id: string,
  expectedInstalledVersion: string | null
) {
  const result = await requestApi<{
    packId: string;
    version: string;
    enabled: boolean;
  }>("POST /api/v1/capabilities/center/releases/{id}/install", {
    pathParams: { id },
    body: { expectedInstalledVersion },
  });
  window.dispatchEvent(new Event("tron:navigation-updated"));
  return result;
}

export interface CenterLikeState {
  packId: string;
  likeCount: number;
  liked: boolean;
}
export function setCenterLike(packId: string, liked: boolean) {
  return requestApi<CenterLikeState>(
    "PUT /api/v1/capabilities/center/packs/{id}/like",
    { pathParams: { id: packId }, body: { liked } }
  );
}
