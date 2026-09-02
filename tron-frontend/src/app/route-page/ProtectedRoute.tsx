import { useAuthSession, buildLoginUrl, NoWorkspaceAccessPage } from "@/domain/auth";
import { ReactNode } from "react";
import { useLocation, Navigate } from "react-router";
import RouteLoadingFallback from "./RouteLoadingFallback";

export default function ProtectedRoute({ children }: { children: ReactNode }) {
  const location = useLocation();
  const { session, status } = useAuthSession();
  const isPlatformAdmin = session?.user.roleCodes.includes("ADMIN") ?? false;

  if (status === "checking") {
    return <RouteLoadingFallback fullScreen />;
  }

  if (status === "unauthenticated") {
    return (
      <Navigate
        replace
        to={buildLoginUrl(`${location.pathname}${location.search}`)}
      />
    );
  }

  if (
    !session?.authenticated ||
    (!isPlatformAdmin && session.user.spaceCodes.length === 0)
  ) {
    return session ? (
      <NoWorkspaceAccessPage session={session} />
    ) : (
      <RouteLoadingFallback fullScreen />
    );
  }

  return <>{children}</>;
}
