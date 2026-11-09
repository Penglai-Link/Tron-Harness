import { UpdateNotification } from "@/domain/update/UpdateNotification";
import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useBootstrapAuthSession } from "@/domain/auth";
import { Navigate, Route, Routes, useNavigate } from "react-router";
import EntryRoutePage from "./route-page/EntryRoutePage";
import ProtectedRoute from "./route-page/ProtectedRoute";
import AppRoutePage from "./route-page/AppRoutePage";
import { TabPanelRegistryProvider } from "@/shared/layout/content/TabPanelRegistry";
import { FilePanelHostProvider } from "@/shared/layout/content/FilePanelHostProvider";
import { ProfileSelectionPage } from "@/domain/profile/pages/ProfileSelectionPage";
import { buildApiUrl } from "@/shared/api/httpClient";
import { subscribeSharedEvent } from "@/shared/api/sharedEventStream";
import { useAuthStore } from "@/store/authStore";

function ServerAuthDeviceUnboundHandler() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  useEffect(() => {
    const onDeviceUnbound = () => {
      useAuthStore.getState().markUnauthenticated();
      queryClient.clear();
      window.dispatchEvent(new Event("server-auth-device-unbound"));
      navigate("/profiles?mode=reauth", { replace: true });
    };
    return subscribeSharedEvent(buildApiUrl("/api/v1/profiles/events"), "server-auth-device-unbound", onDeviceUnbound);
  }, [navigate, queryClient]);

  return null;
}

export default function RoutedApp() {
  useBootstrapAuthSession();

  return (
    <FilePanelHostProvider>
      <TabPanelRegistryProvider>
        <ServerAuthDeviceUnboundHandler />
          <Routes>
            <Route path="/" element={<EntryRoutePage />} />
            <Route path="/awakening" element={<Navigate replace to="/workspace" />} />
            <Route path="/profiles" element={<ProfileSelectionPage />} />
            <Route path="/home" element={<Navigate replace to="/workspace" />} />
            <Route
              path="/design-standard"
              element={<Navigate replace to="/workspace" />}
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
                  <UpdateNotification />
                </ProtectedRoute>
              }
            />
          </Routes>
      </TabPanelRegistryProvider>
    </FilePanelHostProvider>
  );
}
