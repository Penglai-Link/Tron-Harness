import { matchRoutes } from "react-router";
import type { ChatPageContext } from "@tron/chat-core";
import { appRoutes } from "@/router/routes";
import type { TabItem } from "@/store/tabStore";

const routes = appRoutes.map((route) => ({
  path: route.route.split("?")[0],
  handle: route,
}));
const encoder = new TextEncoder();

function compact(value: string | null | undefined, limit: number): string | null {
  const text = Array.from(value?.replace(/\s+/gu, " ").trim() ?? "").slice(0, limit).join("");
  return text || null;
}

/** Called on submission, never on navigation or on queue drain. No page contents are read. */
export function captureDudePageContext({
  pathname, search, tab, workspaceCode, workspaceName, t,
}: {
  pathname: string;
  search: string;
  tab: TabItem | undefined;
  workspaceCode: string | null;
  workspaceName: string | null;
  t: (key: string) => string;
}): ChatPageContext {
  const match = matchRoutes(routes, { pathname })?.at(-1);
  const route = match?.route.handle;
  const title = compact(route?.titleTranslationKey ? t(route.titleTranslationKey) : route?.title, 80);
  // A hidden/default tab or a tab from a different route is not the visible page.
  const visibleTab = tab && !tab.hide && tab.path.split("#")[0] === `${pathname}${search}` ? tab : null;
  const tabTitle = compact(visibleTab?.titleTranslationKey ? t(visibleTab.titleTranslationKey) : visibleTab?.title, 80);
  const locator = route?.pageContextTarget;
  const targetValue = compact(locator?.source === "path"
    ? match?.params[locator.parameter]
    : locator ? new URLSearchParams(search).get(locator.parameter) : null, 128);
  const snapshot: ChatPageContext = {
    title,
    path: compact(pathname.split(/[?#]/u)[0], 160),
    tabTitle: tabTitle !== title ? tabTitle : null,
    workspaceCode: compact(workspaceCode, 128),
    workspaceName: compact(workspaceName, 80),
    target: locator && targetValue ? { key: locator.parameter, value: targetValue } : null,
  };
  // Unicode and JSON escaping count toward the wire budget. Drop optional labels first;
  // keep object identifiers intact so they cannot point at a different object.
  for (const key of ["tabTitle", "workspaceName", "title", "workspaceCode", "target"] as const) {
    if (encoder.encode(JSON.stringify(snapshot)).byteLength <= 2048) break;
    snapshot[key] = null;
  }
  return snapshot;
}
