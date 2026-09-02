import { requestApi } from "@/shared/api/httpClient";
import type {
  NavigationAdminDomain,
  NavigationAdminMenu,
  NavigationAdminPage,
  NavigationAdminTreeItem,
  NavigationLocalizedText,
  NavigationMenuSource,
  NavigationMenuType,
} from "@/shared/types/Navigation";

const navigationAdminApiEndpoints = {
  queryDomains: "POST /api/v1/admin/navigation/domains/query",
  createDomain: "POST /api/v1/admin/navigation/domains",
  getDomain: "GET /api/v1/admin/navigation/domains/{domainCode}",
  updateDomain: "PUT /api/v1/admin/navigation/domains/{domainCode}",
  deleteDomain: "DELETE /api/v1/admin/navigation/domains/{domainCode}",
  queryMenus: "POST /api/v1/admin/navigation/menus/query",
  createMenu: "POST /api/v1/admin/navigation/menus",
  getMenu: "GET /api/v1/admin/navigation/menus/{menuCode}",
  updateMenu: "PUT /api/v1/admin/navigation/menus/{menuCode}",
  deleteMenu: "DELETE /api/v1/admin/navigation/menus/{menuCode}",
  getMenuTree: "GET /api/v1/admin/navigation/domains/{domainCode}/menu-tree",
  addMenuItem: "POST /api/v1/admin/navigation/domains/{domainCode}/menu-items",
  updateMenuItem: "PUT /api/v1/admin/navigation/domains/{domainCode}/menu-items/{menuCode}",
  deleteMenuItem: "DELETE /api/v1/admin/navigation/domains/{domainCode}/menu-items/{menuCode}",
  copyMenuTree: "POST /api/v1/admin/navigation/domains/{targetDomainCode}/menu-tree/copy",
  restoreDefaultMenuTrees: "POST /api/v1/admin/navigation/menu-trees/default",
} as const;

export interface NavigationDomainQuery {
  current: number;
  size: number;
  keyword: string;
  enabled?: boolean;
}

export interface NavigationMenuQuery {
  current: number;
  size: number;
  keyword: string;
  source?: NavigationMenuSource;
  menuType?: NavigationMenuType;
  enabled?: boolean;
}

export interface NavigationDomainPayload {
  domainCode?: string;
  name: NavigationLocalizedText;
  description: NavigationLocalizedText;
  visibleRoles: string[];
  enabled: boolean;
  isDefault: boolean;
  position: number;
}

export interface NavigationMenuPayload {
  menuCode?: string;
  menuType: NavigationMenuType;
  translations: NavigationLocalizedText;
  pageCode: string | null;
  url: string | null;
  icon: string | null;
  enabled: boolean;
}

export interface NavigationMenuItemPayload {
  menuCode?: string;
  parentMenuCode: string | null;
  position: number;
  visibleRoles: string[];
}

export interface NavigationCopyMenuTreePayload {
  sourceDomainCode: string;
  sourceMenuCode: string | null;
  targetParentMenuCode: string | null;
  position: number;
}

export interface NavigationDefaultMenuTreeRestore {
  restoredDomainCodes: string[];
  restoredItemCount: number;
}

function normalizePage<T>(payload: unknown, fallback: Pick<NavigationAdminPage<T>, "current" | "size">): NavigationAdminPage<T> {
  if (typeof payload === "object" && payload !== null && "records" in payload && Array.isArray(payload.records)) {
    const recordPayload = payload as { current?: unknown; size?: unknown; total?: unknown; records: T[] };
    return {
      current: Number(recordPayload.current ?? fallback.current) || fallback.current,
      size: Number(recordPayload.size ?? fallback.size) || fallback.size,
      total: Number(recordPayload.total ?? recordPayload.records.length) || 0,
      records: recordPayload.records,
    };
  }

  const records = Array.isArray(payload) ? payload as T[] : [];
  return { current: fallback.current, size: fallback.size, total: records.length, records };
}

export async function fetchNavigationAdminDomains(query: NavigationDomainQuery, signal?: AbortSignal) {
  const payload = await requestApi<unknown>(navigationAdminApiEndpoints.queryDomains, { body: query, signal });
  return normalizePage<NavigationAdminDomain>(payload, query);
}

export function createNavigationDomain(payload: Required<NavigationDomainPayload>) {
  return requestApi<NavigationAdminDomain>(navigationAdminApiEndpoints.createDomain, { body: payload });
}

export function updateNavigationDomain(domainCode: string, payload: Omit<Required<NavigationDomainPayload>, "domainCode">) {
  return requestApi<NavigationAdminDomain>(navigationAdminApiEndpoints.updateDomain, {
    body: payload,
    pathParams: { domainCode },
  });
}

export function deleteNavigationDomain(domainCode: string) {
  return requestApi<Record<string, never>>(navigationAdminApiEndpoints.deleteDomain, { pathParams: { domainCode } });
}

export async function fetchNavigationAdminMenus(query: NavigationMenuQuery, signal?: AbortSignal) {
  const payload = await requestApi<unknown>(navigationAdminApiEndpoints.queryMenus, { body: query, signal });
  return normalizePage<NavigationAdminMenu>(payload, query);
}

export function createNavigationMenu(payload: Required<NavigationMenuPayload>) {
  return requestApi<NavigationAdminMenu>(navigationAdminApiEndpoints.createMenu, { body: payload });
}

export function updateNavigationMenu(menuCode: string, payload: Omit<Required<NavigationMenuPayload>, "menuCode">) {
  return requestApi<NavigationAdminMenu>(navigationAdminApiEndpoints.updateMenu, {
    body: payload,
    pathParams: { menuCode },
  });
}

export function deleteNavigationMenu(menuCode: string) {
  return requestApi<Record<string, never>>(navigationAdminApiEndpoints.deleteMenu, { pathParams: { menuCode } });
}

export function fetchNavigationAdminMenuTree(domainCode: string, signal?: AbortSignal) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.getMenuTree, {
    pathParams: { domainCode },
    signal,
  });
}

export function addNavigationMenuItem(domainCode: string, payload: Required<NavigationMenuItemPayload>) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.addMenuItem, {
    body: payload,
    pathParams: { domainCode },
  });
}

export function updateNavigationMenuItem(domainCode: string, menuCode: string, payload: Omit<Required<NavigationMenuItemPayload>, "menuCode">) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.updateMenuItem, {
    body: payload,
    pathParams: { domainCode, menuCode },
  });
}

export function deleteNavigationMenuItem(domainCode: string, menuCode: string) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.deleteMenuItem, {
    pathParams: { domainCode, menuCode },
  });
}

export function copyNavigationMenuTree(targetDomainCode: string, payload: NavigationCopyMenuTreePayload) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.copyMenuTree, {
    body: payload,
    pathParams: { targetDomainCode },
  });
}

export function restoreNavigationDefaultMenuTrees() {
  return requestApi<NavigationDefaultMenuTreeRestore>(navigationAdminApiEndpoints.restoreDefaultMenuTrees);
}
