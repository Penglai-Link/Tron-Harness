import { useBootstrapAuthSession } from "@/domain/auth";
import { Navigate, Route, Routes } from "react-router";
import EntryRoutePage from "./route-page/EntryRoutePage";
import ProtectedRoute from "./route-page/ProtectedRoute";
import AppRoutePage from "./route-page/AppRoutePage";
import { TabPanelRegistryProvider } from "@/shared/layout/content/TabPanelRegistry";
import { FilePanelHostProvider } from "@/shared/layout/content/FilePanelHostProvider";
import { ProfileSelectionPage } from "@/domain/profile/pages/ProfileSelectionPage";

export default function RoutedApp() {
  useBootstrapAuthSession();

  return (
    <FilePanelHostProvider>
      <TabPanelRegistryProvider>
        <Routes>
        <Route path="/" element={<EntryRoutePage />} />
        <Route path="/profiles" element={<ProfileSelectionPage />} />
        <Route path="/home" element={<Navigate replace to="/workspace" />} />
        <Route path="/design-standard" element={<Navigate replace to="/workspace" />} />
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
        <Route
          path="*"
          element={
            <ProtectedRoute>
              <AppRoutePage />
            </ProtectedRoute>
          }
        />
        </Routes>
      </TabPanelRegistryProvider>
    </FilePanelHostProvider>
  );
}
