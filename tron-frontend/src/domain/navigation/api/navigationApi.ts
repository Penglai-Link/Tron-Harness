import { requestApi } from "@/shared/api/httpClient";
import type {
  NavigationDomainOption,
  NavigationRuntimeTree,
} from "@/shared/types/Navigation";

const navigationApiEndpoints = {
  listDomains: "GET /api/v1/navigation/domains",
  getMenuTree: "GET /api/v1/navigation/domains/{domainCode}/menu-tree",
  getChatPanelMenuTree: "GET /api/v1/navigation/chat-panel/menu-tree",
} as const;

export function fetchNavigationDomains(signal?: AbortSignal) {
  return requestApi<NavigationDomainOption[]>(navigationApiEndpoints.listDomains, { signal });
}

export function fetchNavigationMenuTree(domainCode: string, signal?: AbortSignal) {
  return requestApi<NavigationRuntimeTree>(navigationApiEndpoints.getMenuTree, {
    pathParams: { domainCode },
    signal,
  });
}

export function fetchChatPanelMenuTree(signal?: AbortSignal) {
  return requestApi<NavigationRuntimeTree>(navigationApiEndpoints.getChatPanelMenuTree, { signal });
}
