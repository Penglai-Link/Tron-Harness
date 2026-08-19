import {
  lazy,
  Suspense,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  Activity,
} from "react";
import {
  ArrowRight,
  LayoutDashboard,
} from "lucide-react";

import { useTranslation } from "react-i18next";
import {
  BrowserRouter,
  matchPath,
  Navigate,
  Route,
  Routes,
  UNSAFE_LocationContext,
  useLocation,
  useNavigationType,
} from "react-router";
import {
  AuthUserMenuPanel,
  NoWorkspaceAccessPage,
  useAuthSession,
  useBootstrapAuthSession,
} from "@/domain/auth";
import {
  SettingsShell,
  ThemeProvider,
  type ConsoleShellConfig,
  ConsoleShellConfigContext,
  TronShell,
  tronNavigation,
  type TronNavItem,
  useToast,
} from "@/shared";
import { AUTH_CALLBACK_PATH } from "@/domain/auth/constants";
import {
  buildLoginUrl,
  resolveAuthRedirect,
} from "@/domain/auth/model";

import { LinkButton } from "@/shared/ui/Button";

import { appRoutes } from "@/router/routes";
import { routePages } from "@/router/pageRegistry";
import { settingsNavigation } from "@/domain/settings/constants/settingsNavigation";
import { useTabStore } from "@/store/tabStore";

const DesignStandardPage = lazy(() => import("@/domain/design-standard/pages/DesignStandardPage").then((module) => ({ default: module.DesignStandardPage })));
const SharedShellPreviewPage = lazy(() => import("@/domain/shared-shell/pages/SharedShellPreviewPage").then((module) => ({ default: module.SharedShellPreviewPage })));

function resolveAppRoute(pathname: string, search: string) {
  const exactRoute = `${pathname}${search}`;
  const route =
    appRoutes.find((item) => item.route === exactRoute) ??
    appRoutes.find((item) => item.route === pathname) ??
    appRoutes.find((item) => {
      const routePathname = new URL(item.route, "https://tron.local").pathname;
      return routePathname.includes(":") && Boolean(matchPath({ end: true, path: routePathname }, pathname));
    });

  const Component = route ? routePages[route.pageId] : undefined;

  return route && Component ? { Component, route } : undefined;
}

const appRoutePaths = Array.from(
  new Set(
    appRoutes
      .filter((route) => route.route !== "/login")
      .map((route) => new URL(route.route, "https://tron.local").pathname),
  ),
);

function flattenNavigationItems(items: TronNavItem[]): TronNavItem[] {
  return items.flatMap((item) => [
    item,
    ...flattenNavigationItems(item.children ?? []),
  ]);
}

const consoleNavigationItems = tronNavigation.flatMap((section) => flattenNavigationItems(section.items));

function resolveActiveItem(pathname: string) {
  const matches = consoleNavigationItems.filter((item) => item.href === pathname);
  return matches.find((item) => !item.children?.length) ?? matches[0];
}

function resolveActiveItemByLabel(label: string) {
  return consoleNavigationItems.find((item) => item.label === label);
}

function isAdministrationRoute(pathname: string) {
  return pathname === "/admin" || pathname.startsWith("/admin/");
}

function usesInPlaceSearch(pageId?: string) {
  return pageId === "chats.index"
    || pageId === "resources.detail"
    || pageId === "settings.resourcesDetail";
}

function routeRenderKeyForLocation(pathname: string, search: string, pageId?: string) {
  return usesInPlaceSearch(pageId) ? pathname : `${pathname}${search}`;
}

function isConsoleShellConfigEqual(left: ConsoleShellConfig, right: ConsoleShellConfig) {
  return (
    left.contentClassName === right.contentClassName
    && left.rightPanel === right.rightPanel
    && left.searchPlaceholder === right.searchPlaceholder
    && left.topbarActions === right.topbarActions
  );
}

function isConsoleShellConfigEmpty(config: ConsoleShellConfig) {
  return !config.contentClassName
    && !config.rightPanel
    && !config.searchPlaceholder
    && !config.topbarActions;
}

function HomePage() {
  const { t } = useTranslation("home");

  return (
    <main className="min-h-screen bg-[var(--color-background)] px-6 py-8 text-[var(--color-on-surface)]">
      <section className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-6xl flex-col justify-center gap-8">
        <div className="max-w-3xl">
          <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-[var(--color-outline-variant)] bg-white px-3 py-1 text-sm text-[var(--color-on-surface-variant)] shadow-sm">
            <LayoutDashboard size={16} aria-hidden="true" />
            {t("badge")}
          </div>
          <h1 className="text-4xl font-semibold leading-tight tracking-normal text-[var(--color-on-surface)]">
            {t("title")}
          </h1>
          <p className="mt-4 max-w-2xl text-base leading-7 text-[var(--color-on-surface-variant)]">
            {t("description")}
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <LinkButton to="/design-standard">
              {t("cta.designStandard")}
              <ArrowRight size={16} aria-hidden="true" />
            </LinkButton>
            <LinkButton to="/shared-shell-preview" variant="secondary">
              {t("cta.sharedShell")}
              <ArrowRight size={16} aria-hidden="true" />
            </LinkButton>
          </div>
        </div>
      </section>
    </main>
  );
}

function ConsoleFallbackPage() {
  const { t } = useTranslation("console");

  return (
    <section className="flex min-h-[calc(100vh-7rem)] items-center justify-center">
      <div className="max-w-xl text-center">
        <h1 className="text-2xl font-semibold">{t("fallback.title")}</h1>
        <p className="mt-3 text-sm leading-6 text-[var(--color-on-surface-variant)]">
          {t("fallback.description")}
        </p>
      </div>
    </section>
  );
}

function RouteLoadingFallback({ fullScreen = false }: { fullScreen?: boolean }) {
  return (
    <section
      aria-busy="true"
      className={fullScreen ? "flex min-h-screen items-center justify-center bg-[var(--color-background)] px-6" : "flex min-h-[calc(100vh-7rem)] items-center justify-center"}
    >
      <div className="h-1.5 w-32 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
        <div className="h-full w-1/2 animate-pulse rounded-full bg-[var(--color-primary)]" />
      </div>
    </section>
  );
}

function SuspendedRoute({ children, fullScreen = false }: { children: ReactNode; fullScreen?: boolean }) {
  return (
    <Suspense fallback={<RouteLoadingFallback fullScreen={fullScreen} />}>
      {children}
    </Suspense>
  );
}

function CachedRouteLocationProvider({ children, path }: { children: ReactNode; path: string }) {
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

function PublicRoutePage({ pageId }: { pageId: string }) {
  const Component = routePages[pageId];

  if (!Component) {
    return <RouteLoadingFallback fullScreen />;
  }

  return (
    <SuspendedRoute fullScreen>
      <Component />
    </SuspendedRoute>
  );
}

function AppRoutePage() {
  const { pathname, search } = useLocation();
  const { session } = useAuthSession();
  const toast = useToast();
  const { t } = useTranslation();
  const addTab = useTabStore((state) => state.addTab);
  const addToCache = useTabStore((state) => state.addToCache);
  const cachedTabsList = useTabStore((state) => state.cachedTabsList);
  const activeTabId = useTabStore((state) => state.activeTabId);
  const resolved = resolveAppRoute(pathname, search);
  const routeRenderKey = routeRenderKeyForLocation(pathname, search, resolved?.route.pageId);
  const isAdmin = session?.user.roleCodes.includes("ADMIN") ?? false;
  const blocksAdministrationRoute = isAdministrationRoute(pathname) && !isAdmin;
  const blockedAdministrationToastPath = useRef<string | null>(null);
  const [pageShellConfig, setPageShellConfig] = useState<ConsoleShellConfig>({});

  // 渲染所有缓存的 tab 内容
  const cachedRouteContents = useMemo(() => {
    return cachedTabsList.map((cachedTab) => {
      // These editors own version selection in the query string. Keep their
      // instance alive while injecting the current query into the active tab.
      const isCurrentTab = cachedTab.tabId === activeTabId;
      const useCurrentSearch = isCurrentTab && usesInPlaceSearch(cachedTab.pageId);

      // cachedTab.path 可能包含查询参数，需要拆分为 pathname 和 search
      const questionMarkIndex = cachedTab.path.indexOf("?");
      const cachedPathname = questionMarkIndex === -1 ? cachedTab.path : cachedTab.path.slice(0, questionMarkIndex);
      const cachedSearch = questionMarkIndex === -1 ? "" : cachedTab.path.slice(questionMarkIndex);
      const searchParam = useCurrentSearch ? search : cachedSearch;

      const cachedResolved = resolveAppRoute(cachedPathname, searchParam);
      if (!cachedResolved) {
        return null;
      }

      const { Component } = cachedResolved;
      const key = routeRenderKeyForLocation(cachedPathname, searchParam, cachedResolved.route.pageId);

      return (
        <Activity key={cachedTab.tabId} mode={isCurrentTab ? "visible" : "hidden"}>
          <CachedRouteLocationProvider path={`${cachedPathname}${searchParam}`}>
            <SuspendedRoute>
              <Component key={key} />
            </SuspendedRoute>
          </CachedRouteLocationProvider>
        </Activity>
      );
    });
  }, [cachedTabsList, activeTabId, search]);

  const shellConfigController = useMemo(() => ({
    resetConfig: () => {
      setPageShellConfig((current) => (isConsoleShellConfigEmpty(current) ? current : {}));
    },
    setConfig: (config: ConsoleShellConfig) => {
      setPageShellConfig((current) => (isConsoleShellConfigEqual(current, config) ? current : config));
    },
  }), []);

  useEffect(() => {
    if (resolved && resolved.route.layout !== "settings") {
      const currentPath = `${pathname}${search}`;
      const navItem = resolveActiveItem(pathname) ?? resolveActiveItemByLabel(resolved.route.activeNav);
      const tabTitle = navItem
        ? t(navItem.translationKey)
        : resolved.route.titleTranslationKey
          ? t(resolved.route.titleTranslationKey)
          : resolved.route.title;
      const activeNav = resolved.route.activeNav;

      // Stateful detail pages react to their query themselves. Replacing their
      // cache entry on every version change races with that page-level reload.
      const shouldIgnoreSearch = usesInPlaceSearch(resolved.route.pageId);
      const lookupPath = shouldIgnoreSearch ? pathname : currentPath;

      // 调用 addTab，它会根据 activeNav 复用已存在的 tab，或创建新 tab
      const tabId = addTab({
        title: tabTitle,
        path: lookupPath,
        closable: true,
        activeNav,
      });

      // 获取这个 tab 的当前状态
      const tab = useTabStore.getState().tabs.find(t => t.id === tabId);

      if (tab) {
        // 检查 path 是否改变
        const pathChanged = tab.path !== lookupPath;

        // 如果 path 改变了，需要清除该 tab 的缓存，让组件重新渲染
        if (pathChanged) {
          useTabStore.getState().removeFromCache(tabId);
        }

        // 更新 tab 的 path 和 title（如果它们与当前路径不一致）
        if (tab.path !== lookupPath) {
          useTabStore.getState().updateTab(tabId, { path: lookupPath });
        }
        if (tab.title !== tabTitle) {
          useTabStore.getState().updateTab(tabId, { title: tabTitle });
        }

        // 设置为当前活动的 tab
        useTabStore.getState().setActiveTab(tabId);

        // 更新缓存
        addToCache(tabId, lookupPath, resolved.route.pageId);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pathname, search, resolved?.route.pageId]);

  useEffect(() => {
    setPageShellConfig((current) => (isConsoleShellConfigEmpty(current) ? current : {}));
  }, [resolved?.route.pageId]);

  useEffect(() => {
    if (!blocksAdministrationRoute) {
      blockedAdministrationToastPath.current = null;
      return;
    }

    if (blockedAdministrationToastPath.current === pathname) {
      return;
    }

    blockedAdministrationToastPath.current = pathname;
    toast.warning("Only Admin role users can access administration pages.", {
      title: "Access restricted",
    });
  }, [blocksAdministrationRoute, pathname, toast]);

  if (blocksAdministrationRoute) {
    return <Navigate replace to="/dashboard" />;
  }

  if (resolved) {
    const { route } = resolved;
    const effectiveShellConfig = {
      ...route.shellConfig,
      ...pageShellConfig,
    };
    if (route.layout === "settings") {
      // Settings 页面不使用 keep-alive，直接渲染
      return (
        <SettingsShell
          activeItem={route.settingsNav ?? route.title}
          contentClassName={route.shellConfig?.contentClassName}
          items={settingsNavigation}
          roleCodes={session?.user.roleCodes}
        >
          <SuspendedRoute>
            <resolved.Component key={routeRenderKey} />
          </SuspendedRoute>
        </SettingsShell>
      );
    }
    return (
      <ConsoleShellConfigContext.Provider value={shellConfigController}>
        <TronShell
          activeItem={route.activeNav}
          contentClassName={effectiveShellConfig.contentClassName}
          rightPanel={effectiveShellConfig.rightPanel}
          searchPlaceholder={effectiveShellConfig.searchPlaceholder}
          roleCodes={session?.user.roleCodes}
          topbarActions={effectiveShellConfig.topbarActions}
          userMenuContent={<AuthUserMenuPanel />}
        >
          {cachedRouteContents}
        </TronShell>
      </ConsoleShellConfigContext.Provider>
    );
  }

  return (
    <TronShell activeItem={resolveActiveItem(pathname)?.label ?? "Dashboard"}>
      <ConsoleFallbackPage />
    </TronShell>
  );
}

function ConsoleFallbackRoutePage() {
  const { pathname } = useLocation();
  const { session } = useAuthSession();

  return (
    <TronShell activeItem={resolveActiveItem(pathname)?.label ?? "Dashboard"} roleCodes={session?.user.roleCodes} userMenuContent={<AuthUserMenuPanel />}>
      <ConsoleFallbackPage />
    </TronShell>
  );
}

function ProtectedRoute({ children }: { children: ReactNode }) {
  const location = useLocation();
  const { session, status } = useAuthSession();
  const isPlatformAdmin = session?.user.roleCodes.includes("ADMIN") ?? false;

  if (status === "checking") {
    return <RouteLoadingFallback fullScreen />;
  }

  if (status === "unauthenticated") {
    return <Navigate replace to={buildLoginUrl(`${location.pathname}${location.search}`)} />;
  }

  if (!session?.authenticated || (!isPlatformAdmin && session.user.spaceCodes.length === 0)) {
    return session ? <NoWorkspaceAccessPage session={session} /> : <RouteLoadingFallback fullScreen />;
  }

  return <>{children}</>;
}

function LoginRoutePage() {
  const location = useLocation();
  const { session, status } = useAuthSession();

  if (status === "checking") {
    return <RouteLoadingFallback fullScreen />;
  }

  if (session?.authenticated) {
    return <Navigate replace to={resolveAuthRedirect(location.search)} />;
  }

  return <PublicRoutePage pageId="auth.login" />;
}

function RoutedApp() {
  useBootstrapAuthSession();

  return (
    <Routes>
      <Route path="/" element={<LoginRoutePage />} />
      <Route path="/login" element={<LoginRoutePage />} />
      <Route path={AUTH_CALLBACK_PATH} element={<PublicRoutePage pageId="auth.callback" />} />
      <Route
        path="/home"
        element={(
          <ProtectedRoute>
            <HomePage />
          </ProtectedRoute>
        )}
      />
      <Route
        path="/design-standard"
        element={(
          <ProtectedRoute>
            <SuspendedRoute fullScreen>
              <DesignStandardPage />
            </SuspendedRoute>
          </ProtectedRoute>
        )}
      />
      <Route
        path="/shared-shell-preview"
        element={(
          <ProtectedRoute>
            <SuspendedRoute fullScreen>
              <SharedShellPreviewPage />
            </SuspendedRoute>
          </ProtectedRoute>
        )}
      />
      <Route
        path="/settings"
        element={(
          <ProtectedRoute>
            <Navigate replace to="/settings/general" />
          </ProtectedRoute>
        )}
      />
      <Route
        path="/admin/platform/models"
        element={(
          <ProtectedRoute>
            <Navigate replace to="/settings/models" />
          </ProtectedRoute>
        )}
      />
      <Route
        path="/skills/installed"
        element={(
          <ProtectedRoute>
            <Navigate replace to="/settings/skills" />
          </ProtectedRoute>
        )}
      />
      <Route
        path="/skills/installed/new"
        element={(
          <ProtectedRoute>
            <Navigate replace to="/settings/skills/sync" />
          </ProtectedRoute>
        )}
      />
      <Route
        path="/skills/new"
        element={(
          <ProtectedRoute>
            <Navigate replace to="/settings/skills/sync" />
          </ProtectedRoute>
        )}
      />
      <Route
        path="/resource-management/files/new"
        element={(
          <ProtectedRoute>
            <Navigate replace to="/resource-management/files" />
          </ProtectedRoute>
        )}
      />
      {appRoutePaths.map((path) => (
        <Route
          element={(
            <ProtectedRoute>
              <AppRoutePage />
            </ProtectedRoute>
          )}
          key={path}
          path={path}
        />
      ))}
      <Route
        path="*"
        element={(
          <ProtectedRoute>
            <ConsoleFallbackRoutePage />
          </ProtectedRoute>
        )}
      />
    </Routes>
  );
}

export function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <RoutedApp />
      </ThemeProvider>
    </BrowserRouter>
  );
}
