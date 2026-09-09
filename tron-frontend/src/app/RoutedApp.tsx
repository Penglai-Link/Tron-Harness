import { AUTH_CALLBACK_PATH, useBootstrapAuthSession } from "@/domain/auth";
import { Navigate, Route, Routes } from "react-router";
import ConsoleFallbackRoutePage from "./route-page/ConsoleFallbackRoutePage";
import EntryRoutePage from "./route-page/EntryRoutePage";
import HomePage from "./route-page/HomePage";
import PublicRoutePage from "./route-page/PublicRoutePage";
import SuspendedRoute from "./route-page/SuspendedRoute";
import ProtectedRoute from "./route-page/ProtectedRoute";
import { appRoutes } from "@/router/routes";
import AppRoutePage from "./route-page/AppRoutePage";
import { lazy } from "react";
import { TabPanelRegistryProvider } from "@/shared/layout/content/TabPanelRegistry";

const DesignStandardPage = lazy(() =>
  import("@/domain/design-standard/pages/DesignStandardPage").then(
    (module) => ({ default: module.DesignStandardPage })
  )
);
const SetupPage = lazy(() =>
  import("@/domain/setup/pages/SetupPage").then((module) => ({
    default: module.SetupPage,
  }))
);

const appRoutePaths = Array.from(
  new Set(
    appRoutes
      .filter((route) => route.route !== "/login")
      .map((route) => new URL(route.route, "https://tron.local").pathname)
  )
);

export default function RoutedApp() {
  useBootstrapAuthSession();

  return (
    <TabPanelRegistryProvider>
      <Routes>
        <Route path="/login" element={<EntryRoutePage />} />
        <Route
          path="/setup"
          element={
            <SuspendedRoute fullScreen>
              <SetupPage />
            </SuspendedRoute>
          }
        />
        <Route
          path={AUTH_CALLBACK_PATH}
          element={<PublicRoutePage pageId="auth.callback" />}
        />
        <Route
          path="/home"
          element={
            <ProtectedRoute>
              <HomePage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/design-standard"
          element={
            <ProtectedRoute>
              <SuspendedRoute fullScreen>
                <DesignStandardPage />
              </SuspendedRoute>
            </ProtectedRoute>
          }
        />
        <Route
          path="/settings"
          element={
            <ProtectedRoute>
              <Navigate replace to="/settings/general" />
            </ProtectedRoute>
          }
        />
        <Route
          path="/admin/platform/models"
          element={
            <ProtectedRoute>
              <Navigate replace to="/settings/models" />
            </ProtectedRoute>
          }
        />
        <Route
          path="/skills/installed"
          element={
            <ProtectedRoute>
              <Navigate replace to="/settings/skills" />
            </ProtectedRoute>
          }
        />
        <Route
          path="/skills/installed/new"
          element={
            <ProtectedRoute>
              <Navigate replace to="/settings/skills/sync" />
            </ProtectedRoute>
          }
        />
        <Route
          path="/skills/new"
          element={
            <ProtectedRoute>
              <Navigate replace to="/settings/skills/sync" />
            </ProtectedRoute>
          }
        />
        <Route
          path="/resource-management/files/new"
          element={
            <ProtectedRoute>
              <Navigate replace to="/resource-management/files" />
            </ProtectedRoute>
          }
        />
        {appRoutePaths.map((path) => (
          <Route
            element={
              <ProtectedRoute>
                <AppRoutePage />
              </ProtectedRoute>
            }
            key={path}
            path={path}
          />
        ))}
        <Route
          path="*"
          element={
            <ProtectedRoute>
              <ConsoleFallbackRoutePage />
            </ProtectedRoute>
          }
        />
      </Routes>
    </TabPanelRegistryProvider>
  );
}
