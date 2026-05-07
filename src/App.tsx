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
} from "@/components/tron";

import { Button } from "@/components/ui/button";

import { appRoutes } from "@/router/app-routes";
import { DesignStandardPage } from "@/domain/design-standard/page";
import {
  LoginPage,
  routePages,
  TasksImportPage,
} from "@/router/page-registry";

import { SharedShellPreviewPage } from "@/domain/shared-shell/page/preview";

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

function AppRoutePage() {
  const { pathname, search } = useLocation();
  const resolved = resolveAppRoute(pathname, search);
  if (resolved) {
    const { Component, route } = resolved;
    if (route.pageId === "auth.login") {
      return <Component />;
    }

    return (
      <TronShell activeItem={route.activeNav}>
        <Component />
      </TronShell>
    );
  }

  return (
    <TronShell activeItem={resolveActiveItem(pathname)}>
      <ConsoleFallbackPage />
    </TronShell>
  );
}

function TasksImportRoutePage() {
  return (
    <TronShell activeItem="Tasks">
      <TasksImportPage />
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
      <Route path="/" element={<LoginPage />} />
      <Route path="/login" element={<LoginPage />} />
      <Route path="/home" element={<HomePage />} />
      <Route path="/design-standard" element={<DesignStandardPage />} />
      <Route path="/shared-shell-preview" element={<SharedShellPreviewPage />} />
      <Route path="/tasks/import" element={<TasksImportRoutePage />} />
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
