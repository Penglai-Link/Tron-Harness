import { PlatformWorkspaceScope } from "@/domain/settings/components/PlatformWorkspaceScope";
import { AuthUserMenuPanel } from "@/domain/auth";
import { settingsNavigation } from "@/domain/settings/constants/settingsNavigation";
import { SettingsShell } from "@/shared/layout/SettingsShell";
import { TronShell } from "@/shared/layout/Shell";
import { useChatSessionLifecycle } from "@/shared/layout/chat/hooks/useChatSessionLifecycle";
import { TabPanelRegistryProvider } from "@/shared/layout/content/TabPanelRegistry";
import { useTabStore } from "@/store/tabStore";
import { useAppShellStore } from "@/store/appShellStore";
import { WorkspaceTabScope } from "@/shared/layout/content/WorkspaceTabScope";
import { MotionReveal } from "@/shared/motion";
import {
  useMemo,
  useEffect,
  ReactNode,
} from "react";
import { useTranslation } from "react-i18next";
import {
  matchPath,
  Navigate,
  Route,
  Routes,
  UNSAFE_LocationContext,
  useLocation,
  useNavigate,
  useNavigationType,
} from "react-router";
import ConsoleFallbackPage from "./ConsoleFallbackPage";
import SuspendedRoute from "./SuspendedRoute";
import { appRoutes } from "@/router/routes";
import { routePages } from "@/router/pageRegistry";
import { consoleNavigationItems, resolveActiveItem } from "./ConsoleFallbackRoutePage";

function usesInPlaceSearch(pageId?: string) {
  return (
    pageId === "resources.detail" ||
    pageId === "settings.resourcesDetail"
  );
}

function activeTabOwnsLocation(activeTabPath: string | undefined, pathname: string) {
  if (!activeTabPath) return false;
  return new URL(activeTabPath, "https://tron.local").pathname === pathname;
}

function routeRenderKeyForLocation(
  pathname: string,
  search: string,
  pageId?: string,
) {
  if (pageId === "settings.variablesDetail") {
    // Changing edit mode must not replace the form while the user is typing.
    // A different variable still gets a fresh component and draft.
    const parameters = new URLSearchParams(search);
    parameters.delete("mode");
    return `${pathname}?${parameters.toString()}`;
  }
  return usesInPlaceSearch(pageId) ? pathname : `${pathname}${search}`;
}

function resolveAppRoute(pathname: string, search: string) {
  const exactRoute = `${pathname}${search}`;
  const route =
    appRoutes.find((item) => item.route === exactRoute) ??
    appRoutes.find((item) => item.route === pathname) ??
    appRoutes.find((item) => {
      const routePathname = new URL(item.route, "https://tron.local").pathname;
      return (
        routePathname.includes(":") &&
        Boolean(matchPath({ end: true, path: routePathname }, pathname))
      );
    });

  const Component = route ? routePages[route.pageId] : undefined;

  return route && Component ? { Component, route } : undefined;
}

function resolveActiveItemByLabel(label: string) {
  return consoleNavigationItems.find((item) => item.label === label);
}

function CachedRouteLocationProvider({
  children,
  path,
}: {
  children: ReactNode;
  path: string;
}) {
  const navigationType = useNavigationType();
  const location = useMemo(() => {
    const url = new URL(path, "https://tron.local");

    return {
      hash: url.hash,
      key: `cached:${path}`,
      pathname: url.pathname,
      search: url.search,
      state: null,
    };
  }, [path]);

  return (
    <UNSAFE_LocationContext.Provider value={{ location, navigationType }}>
      {children}
    </UNSAFE_LocationContext.Provider>
  );
}

export default function AppRoutePage() {
  useEffect(() => {
    void useAppShellStore.getState().loadShortcutDefaults().catch((error: unknown) => {
      console.warn("Unable to load shortcut defaults; using bundled defaults", error);
    });
  }, []);

  useChatSessionLifecycle();
  const { pathname, search, key: locationKey } = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const observabilityWorkspace = pathname.startsWith("/settings/observability/") ? new URLSearchParams(search).get("workspaceCode") : null;
  const scopedSettingsNavigation = useMemo(() => settingsNavigation.map((group) => group.groupKey === "observability" && observabilityWorkspace
    ? { ...group, items: group.items.map((item) => ({ ...item, href: `${item.href}?${new URLSearchParams({ workspaceCode: observabilityWorkspace })}` })) }
    : group), [observabilityWorkspace]);
  const chatSessionCode =
    pathname === "/chats"
      ? new URLSearchParams(search).get("sessionCode")?.trim() ?? ""
      : "";
  const chatDraftCode = pathname === "/chats"
    ? new URLSearchParams(search).get("draftCode")?.trim() ?? ""
    : "";
  const isBareChatsRoute = pathname === "/chats" && !chatSessionCode && !chatDraftCode;

  const addTab = useTabStore((state) => state.addTab);
  const addToCache = useTabStore((state) => state.addToCache);
  const cachedTabsList = useTabStore((state) => state.cachedTabsList);
  const activeTabId = useTabStore((state) => state.activeTabId);
  const workspaceCode = useTabStore((state) => state.workspaceCode);
  const pendingWorkspacePath = useTabStore((state) => state.pendingWorkspacePath);
  const selectedWorkspaceCode = useAppShellStore((state) => state.selectedSecondarySpaceId || state.selectedPrimarySpaceId);
  const activeTabPath = useTabStore((state) =>
    state.tabs.find((tab) => tab.id === state.activeTabId)?.path,
  );
  const useActiveLocationSearch = activeTabOwnsLocation(activeTabPath, pathname);
  const resolved = resolveAppRoute(pathname, search);
  const routeRenderKey = routeRenderKeyForLocation(
    pathname,
    search,
    resolved?.route.pageId,
  );
  // 渲染所有缓存的 tab 内容
  const cachedRouteContents = useMemo(() => {
    return cachedTabsList.map((cachedTab) => {
      // These editors own version selection in the query string. Keep their
      // instance alive while injecting the current query into the active tab.
      const isCurrentTab = cachedTab.tabId === activeTabId && cachedTab.workspaceCode === workspaceCode;
      const useCurrentSearch =
        isCurrentTab &&
        usesInPlaceSearch(cachedTab.pageId) &&
        useActiveLocationSearch;

      // cachedTab.path 可能包含查询参数，需要拆分为 pathname 和 search
      const questionMarkIndex = cachedTab.path.indexOf("?");
      const cachedPathname =
        questionMarkIndex === -1
          ? cachedTab.path
          : cachedTab.path.slice(0, questionMarkIndex);
      const cachedSearch =
        questionMarkIndex === -1 ? "" : cachedTab.path.slice(questionMarkIndex);
      const searchParam = useCurrentSearch ? search : cachedSearch;

      const cachedResolved = resolveAppRoute(cachedPathname, searchParam);
      if (!cachedResolved) {
        return null;
      }

      const { Component, route: cachedRoute } = cachedResolved;
      return (
        <MotionReveal
          key={`${cachedTab.tabId}:${cachedTab.revision}`}
          hidden={!isCurrentTab}
          disabled={!isCurrentTab}
          preset="fade"
          replayKey={isCurrentTab ? "active" : "cached"}
          className="h-full"
        >
          <WorkspaceTabScope workspaceCode={cachedTab.workspaceCode}>
            <TabPanelRegistryProvider
              key={cachedTab.tabId}
              tabId={cachedTab.tabId}
            >
              <CachedRouteLocationProvider path={`${cachedPathname}${searchParam}`}>
                <Routes>
                  <Route path={new URL(cachedRoute.route, "https://tron.local").pathname} element={<SuspendedRoute><Component /></SuspendedRoute>} />
                </Routes>
              </CachedRouteLocationProvider>
            </TabPanelRegistryProvider>
          </WorkspaceTabScope>
        </MotionReveal>
      );
    });
  }, [activeTabId, cachedTabsList, search, useActiveLocationSearch, workspaceCode]);

  useEffect(() => {
    const currentPath = `${pathname}${search}`;
    const state = useTabStore.getState();
    const selection = useAppShellStore.getState();
    const requestedWorkspace = new URLSearchParams(search).get("workspaceCode");
    const ownerWorkspace = state.getWorkspaceForPath(currentPath);
    // The shell resolves workspace navigation first. Never insert the previous
    // URL into the new space while its router transition is still pending.
    if (!workspaceCode || state.workspaceCode !== workspaceCode
      || workspaceCode !== (selection.selectedSecondarySpaceId || selection.selectedPrimarySpaceId)
      || (state.pendingWorkspacePath && state.pendingWorkspacePath !== currentPath)
      || (requestedWorkspace && requestedWorkspace !== workspaceCode)
      || (ownerWorkspace && ownerWorkspace !== workspaceCode)) return;
    state.finishWorkspaceSwitch(currentPath);
    if (pathname === "/chats" && (chatSessionCode || chatDraftCode) && !requestedWorkspace) {
      const params = new URLSearchParams(search);
      params.set("workspaceCode", workspaceCode);
      navigate(`${pathname}?${params}`, { replace: true });
      return;
    }
    if (!isBareChatsRoute && resolved && resolved.route.layout !== "settings") {
      const requestedSessionCode =
        resolved.route.pageId === "chats.index" ? chatSessionCode : "";
      const sessionActiveNav = requestedSessionCode
        ? `chat-session:${requestedSessionCode}`
        : resolved.route.pageId === "chats.index" && chatDraftCode
          ? `chat-draft:${chatDraftCode}`
          : undefined;
      const existingSessionTab = sessionActiveNav
        ? useTabStore
            .getState()
            .tabs.find((tab) => tab.activeNav === sessionActiveNav)
        : undefined;
      const navItem =
        resolveActiveItem(pathname) ??
        resolveActiveItemByLabel(resolved.route.activeNav);
      const titleTranslationKey = chatDraftCode && !requestedSessionCode
        ? "shell:chatSidebar.newChat"
        : sessionActiveNav
        ? undefined
        : navItem?.translationKey ?? resolved.route.titleTranslationKey;
      const tabTitle = chatDraftCode && !requestedSessionCode
        ? t("shell:chatSidebar.newChat")
        : sessionActiveNav
        ? existingSessionTab?.title ?? "New chat"
        : titleTranslationKey
          ? t(titleTranslationKey)
          : resolved.route.title;
      const activeNav = sessionActiveNav ?? resolved.route.activeNav;

      // Stateful detail pages react to their query themselves. Replacing their
      // cache entry on every version change races with that page-level reload.
      const shouldIgnoreSearch = usesInPlaceSearch(resolved.route.pageId);
      const lookupPath =
        shouldIgnoreSearch && !sessionActiveNav ? pathname : currentPath;

      // 调用 addTab，它会根据 activeNav 复用已存在的 tab，或创建新 tab
      const tabId = addTab({
        title: tabTitle,
        titleTranslationKey,
        path: lookupPath,
        closable: true,
        activeNav,
      });

      // 获取这个 tab 的当前状态
      const tab = useTabStore.getState().tabs.find((t) => t.id === tabId);

      if (tab) {
        // 检查 path 是否改变
        const pathChanged = tab.path !== lookupPath;

        // 如果 path 改变了，需要清除该 tab 的缓存，让组件重新渲染
        const previousUrl = new URL(tab.path, "https://tron.local");
        const nextUrl = new URL(lookupPath, "https://tron.local");
        previousUrl.searchParams.delete("workspaceCode");
        nextUrl.searchParams.delete("workspaceCode");
        if (pathChanged && previousUrl.toString() !== nextUrl.toString()) {
          useTabStore.getState().removeFromCache(tabId);
        }

        // 更新 tab 的 path 和 title（如果它们与当前路径不一致）
        if (tab.path !== lookupPath) {
          useTabStore.getState().updateTab(tabId, { path: lookupPath });
        }
        if (
          tab.title !== tabTitle ||
          tab.titleTranslationKey !== titleTranslationKey
        ) {
          useTabStore
            .getState()
            .updateTab(tabId, { title: tabTitle, titleTranslationKey });
        }

        // 设置为当前活动的 tab
        useTabStore.getState().setActiveTab(tabId);

        // 更新缓存
        addToCache(tabId, lookupPath, resolved.route.pageId);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    pathname,
    search,
    // 关闭最后一个标签页会保留 URL；同地址再次导航也需要重建页面缓存。
    locationKey,
    resolved?.route.pageId,
    workspaceCode,
    selectedWorkspaceCode,
    pendingWorkspacePath,
    navigate,
  ]);

  if (isBareChatsRoute) {
    return <Navigate replace to="/workspace" />;
  }

  if (/^\/observability(?:\/|$)/.test(pathname)) {
    return <Navigate replace to={`${pathname === "/observability" ? "/settings/observability/dashboard" : `/settings${pathname}`}${search}`} />;
  }

  if (resolved?.route.layout === "settings") {
    const { route } = resolved;
    // Settings 页面不使用 keep-alive，直接渲染
    return (
      <SettingsShell
        activeItem={route.settingsNav ?? route.title}
        contentClassName={route.shellConfig?.contentClassName}
        items={scopedSettingsNavigation}
      >
        <MotionReveal preset="fade" replayKey={pathname} className="h-full">
        <Routes>
          <Route path={new URL(route.route, "https://tron.local").pathname} element={<SuspendedRoute>{ /^(memory|knowledgeGraph|observability)\./.test(route.pageId)
            ? <PlatformWorkspaceScope inlineFilter={["observability.dashboard", "observability.logs", "observability.metrics", "observability.traces", "observability.traceDetail"].includes(route.pageId)} showScopeHelp={!["observability.dashboard", "observability.logs", "observability.traces", "observability.metrics"].includes(route.pageId)} allowAll={route.pageId.startsWith("observability.")} renderWithoutWorkspace={route.pageId === "memory.index" || route.pageId === "knowledgeGraph.index"}><resolved.Component key={routeRenderKey} /></PlatformWorkspaceScope>
            : <resolved.Component key={routeRenderKey} /> }</SuspendedRoute>} />
        </Routes>
        </MotionReveal>
      </SettingsShell>
    );
  }

  const route = resolved?.route;
  return (
    <TronShell
      activeItem={
        route?.activeNav ?? resolveActiveItem(pathname)?.label ?? "Apps"
      }
      contentClassName={route?.shellConfig?.contentClassName}
      searchPlaceholder={
        route?.shellConfig?.searchPlaceholder
          ? t("shell:placeholders.search")
          : undefined
      }
      userMenuContent={<AuthUserMenuPanel />}
    >
      {cachedRouteContents}
      {!resolved && pathname !== "/workspace" ? <ConsoleFallbackPage /> : null}
    </TronShell>
  );
}
