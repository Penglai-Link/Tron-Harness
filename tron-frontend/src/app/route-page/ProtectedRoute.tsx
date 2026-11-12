import { useAuthSession } from "@/domain/auth";
import { ReactNode } from "react";
import { Navigate } from "react-router";
import RouteLoadingFallback from "./RouteLoadingFallback";
import { useProfileAgreementStore } from "@/store/profileAgreementStore";

export default function ProtectedRoute({ children }: { children: ReactNode }) {
  const { session, status } = useAuthSession();
  const agreementChecked = useProfileAgreementStore((state) => state.checked);

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

  if (!session?.authenticated || !agreementChecked) {
    return <Navigate replace to="/profiles" />;
  }

  return <>{children}</>;
}
