import { requestApi } from "@/shared/api/httpClient";
import type {
  NavigationAdminDomain,
  NavigationAdminMenu,
  NavigationAdminMenuTree,
  NavigationAdminPage,
  NavigationAdminTreeItem,
  NavigationLocalizedText,
  NavigationMenuSource,
  NavigationMenuChildDisplay,
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
  listMenuTrees: "GET /api/v1/admin/navigation/menu-trees",
  getMenuTree: "GET /api/v1/admin/navigation/menu-trees/{treeCode}",
  addMenuItem: "POST /api/v1/admin/navigation/menu-trees/{treeCode}/menu-items",
  updateMenuItem: "PUT /api/v1/admin/navigation/menu-trees/{treeCode}/menu-items/{menuCode}",
  deleteMenuItem: "DELETE /api/v1/admin/navigation/menu-trees/{treeCode}/menu-items/{menuCode}",
  copyMenuTree: "POST /api/v1/admin/navigation/menu-trees/{targetTreeCode}/copy",
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
  enabled: boolean;
  isDefault: boolean;
  position: number;
}

export interface NavigationMenuPayload {
  menuCode?: string;
  menuType: NavigationMenuType;
  translations: NavigationLocalizedText;
  description: NavigationLocalizedText;
  pageCode: string | null;
  url: string | null;
  icon: string | null;
  enabled: boolean;
}

export interface NavigationMenuItemPayload {
  menuCode?: string;
  parentMenuCode: string | null;
  position: number;
}

export interface NavigationMenuItemUpdatePayload {
  parentMenuCode: string | null;
  position: number;
  childDisplay?: NavigationMenuChildDisplay;
}

export interface NavigationCopyMenuTreePayload {
  sourceTreeCode: string;
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

export function fetchNavigationAdminMenuTrees(signal?: AbortSignal) {
  return requestApi<NavigationAdminMenuTree[]>(navigationAdminApiEndpoints.listMenuTrees, { signal });
}

export function createNavigationDomain(payload: Omit<Required<NavigationDomainPayload>, "position" | "domainCode">) {
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

export function fetchNavigationAdminMenuTree(treeCode: string, signal?: AbortSignal) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.getMenuTree, {
    pathParams: { treeCode },
    signal,
  });
}

export function addNavigationMenuItem(treeCode: string, payload: Required<NavigationMenuItemPayload>) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.addMenuItem, {
    body: payload,
    pathParams: { treeCode },
  });
}

export function updateNavigationMenuItem(treeCode: string, menuCode: string, payload: NavigationMenuItemUpdatePayload) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.updateMenuItem, {
    body: payload,
    pathParams: { treeCode, menuCode },
  });
}

export function deleteNavigationMenuItem(treeCode: string, menuCode: string) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.deleteMenuItem, {
    pathParams: { treeCode, menuCode },
  });
}

export function copyNavigationMenuTree(targetTreeCode: string, payload: NavigationCopyMenuTreePayload) {
  return requestApi<NavigationAdminTreeItem[]>(navigationAdminApiEndpoints.copyMenuTree, {
    body: payload,
    pathParams: { targetTreeCode },
  });
}

export function restoreNavigationDefaultMenuTrees() {
  return requestApi<NavigationDefaultMenuTreeRestore>(navigationAdminApiEndpoints.restoreDefaultMenuTrees);
}
