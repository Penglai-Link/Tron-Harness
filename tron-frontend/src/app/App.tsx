import {
  lazy,
  Suspense,
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
  Navigate,
  Route,
  Routes,
  useLocation,
} from "react-router";
import {
  TronShell,
  tronNavigation,
} from "@/shared";

import { Button } from "@/shared/ui/Button";

import { appRoutes } from "@/router/routes";
import { routePages } from "@/router/pageRegistry";

const DesignStandardPage = lazy(() => import("@/domain/design-standard/pages/DesignStandardPage").then((module) => ({ default: module.DesignStandardPage })));
const SharedShellPreviewPage = lazy(() => import("@/domain/shared-shell/pages/SharedShellPreviewPage").then((module) => ({ default: module.SharedShellPreviewPage })));

function resolveAppRoute(pathname: string, search: string) {
  const exactRoute = `${pathname}${search}`;
  const route =
    appRoutes.find((item) => item.route === exactRoute) ??
    appRoutes.find((item) => item.route === pathname) ??
    (pathname === "/chats" ? appRoutes.find((item) => item.route === "/chats/8842-alpha") : undefined);

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
            <Button asChild>
              <Link to="/design-standard">
                {t("cta.designStandard")}
                <ArrowRight size={16} aria-hidden="true" />
              </Link>
            </Button>
            <Button asChild variant="secondary">
              <Link to="/shared-shell-preview">
                {t("cta.sharedShell")}
                <ArrowRight size={16} aria-hidden="true" />
              </Link>
            </Button>
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
  const resolved = resolveAppRoute(pathname, search);
  if (resolved) {
    const { Component, route } = resolved;
    if (route.pageId === "auth.login") {
      return (
        <SuspendedRoute fullScreen>
          <Component />
        </SuspendedRoute>
      );
    }

    return (
      <TronShell activeItem={route.activeNav}>
        <SuspendedRoute>
          <Component />
        </SuspendedRoute>
      </TronShell>
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

  return (
    <TronShell activeItem={resolveActiveItem(pathname)}>
      <ConsoleFallbackPage />
    </TronShell>
  );
}

function RoutedApp() {
  return (
    <Routes>
      <Route path="/" element={<PublicRoutePage pageId="auth.login" />} />
      <Route path="/login" element={<PublicRoutePage pageId="auth.login" />} />
      <Route path="/home" element={<HomePage />} />
      <Route path="/design-standard" element={<SuspendedRoute fullScreen><DesignStandardPage /></SuspendedRoute>} />
      <Route path="/shared-shell-preview" element={<SuspendedRoute fullScreen><SharedShellPreviewPage /></SuspendedRoute>} />
      <Route path="/chats" element={<Navigate replace to="/chats/8842-alpha" />} />
      {appRoutePaths.map((path) => (
        <Route element={<AppRoutePage />} key={path} path={path} />
      ))}
      <Route path="*" element={<ConsoleFallbackRoutePage />} />
    </Routes>
  );
}

export function App() {
  return (
    <BrowserRouter>
      <RoutedApp />
    </BrowserRouter>
  );
}
