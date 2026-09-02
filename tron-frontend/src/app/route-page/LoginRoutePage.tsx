import { useAuthSession, resolveAuthRedirect } from "@/domain/auth";
import { useLocation, Navigate } from "react-router";
import RouteLoadingFallback from "./RouteLoadingFallback";
import PublicRoutePage from "./PublicRoutePage";
import ProtectedRoute from "./ProtectedRoute";
import HomePage from "./HomePage";

export default function LoginRoutePage() {
  const location = useLocation();
  const { session, status } = useAuthSession();

  if (status === "checking") {
    return <RouteLoadingFallback fullScreen />;
  }

  if (session?.authenticated) {
    const redirectTarget = resolveAuthRedirect(location.search);

    if (redirectTarget !== "/" || location.pathname !== "/") {
      return <Navigate replace to={redirectTarget} />;
    }

    return (
      <ProtectedRoute>
        <HomePage />
      </ProtectedRoute>
    );
  }

  return <PublicRoutePage pageId="auth.login" />;
}
