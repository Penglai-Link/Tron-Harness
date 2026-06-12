import {
  lazy,
  Suspense,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  ArrowRight,
  LayoutDashboard,
} from "lucide-react";

import { useTranslation } from "react-i18next";
import {
  BrowserRouter,
  Link,
  matchPath,
  Navigate,
  Route,
  Routes,
  useLocation,
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
  ToastProvider,
  type ConsoleShellConfig,
  ConsoleShellConfigContext,
  TronShell,
  tronNavigation,
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

function resolveActiveItem(pathname: string) {
  return tronNavigation.flatMap((section) => section.items).find((item) => item.href === pathname)?.label ?? "Dashboard";
}

function isAdministrationRoute(pathname: string) {
  return pathname === "/admin" || pathname.startsWith("/admin/");
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
  const resolved = resolveAppRoute(pathname, search);
  const routeRenderKey = `${pathname}${search}`;
  const isAdmin = session?.user.roleCodes.includes("ADMIN") ?? false;
  const blocksAdministrationRoute = isAdministrationRoute(pathname) && !isAdmin;
  const blockedAdministrationToastPath = useRef<string | null>(null);
  const [pageShellConfig, setPageShellConfig] = useState<ConsoleShellConfig>({});
  const shellConfigController = useMemo(() => ({
    resetConfig: () => {
      setPageShellConfig((current) => (isConsoleShellConfigEmpty(current) ? current : {}));
    },
    setConfig: (config: ConsoleShellConfig) => {
      setPageShellConfig((current) => (isConsoleShellConfigEqual(current, config) ? current : config));
    },
  }), []);

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
    const { Component, route } = resolved;
    const effectiveShellConfig = {
      ...route.shellConfig,
      ...pageShellConfig,
    };
    if (route.layout === "settings") {
      return (
        <SettingsShell activeItem={route.settingsNav ?? route.title} items={settingsNavigation}>
          <SuspendedRoute>
            <Component key={routeRenderKey} />
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
          <SuspendedRoute>
            <Component key={routeRenderKey} />
          </SuspendedRoute>
        </TronShell>
      </ConsoleShellConfigContext.Provider>
    );
  }

  return (
    <TronShell activeItem={resolveActiveItem(pathname)}>
      <ConsoleFallbackPage />
    </TronShell>
  );
}

function ConsoleFallbackRoutePage() {
  const { pathname } = useLocation();
  const { session } = useAuthSession();

  return (
    <TronShell activeItem={resolveActiveItem(pathname)} roleCodes={session?.user.roleCodes} userMenuContent={<AuthUserMenuPanel />}>
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
        <ToastProvider>
          <RoutedApp />
        </ToastProvider>
      </ThemeProvider>
    </BrowserRouter>
  );
}
