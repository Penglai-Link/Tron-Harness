import { useAuthSession } from "@/domain/auth";
import { ReactNode } from "react";
import { Navigate } from "react-router";
import RouteLoadingFallback from "./RouteLoadingFallback";

export default function ProtectedRoute({ children }: { children: ReactNode }) {
  const { session, status } = useAuthSession();

  if (status === "checking") {
    return <RouteLoadingFallback fullScreen />;
  }

  if (status === "unauthenticated") {
    return (
      <Navigate
        replace
        to="/profiles"
      />
    );
  }

  if (!session?.authenticated) {
    return <Navigate replace to="/profiles" />;
  }

  return <>{children}</>;
}
