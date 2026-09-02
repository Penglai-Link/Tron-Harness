import { useAuthSession, resolveAuthRedirect } from "@/domain/auth";
import { useLocation, Navigate } from "react-router";
import RouteLoadingFallback from "./RouteLoadingFallback";
import PublicRoutePage from "./PublicRoutePage";

export default function LoginRoutePage() {
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
