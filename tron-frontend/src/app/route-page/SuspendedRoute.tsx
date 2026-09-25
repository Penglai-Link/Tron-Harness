import { ReactNode, Suspense } from "react";
import RouteLoadingFallback from "./RouteLoadingFallback";
import { useLocation } from "react-router";
import { RenderErrorBoundary } from "../RenderErrorBoundary";

export default function SuspendedRoute({
  children,
  fullScreen = false,
}: {
  children: ReactNode;
  fullScreen?: boolean;
}) {
  const { pathname, search } = useLocation();
  return (
    <RenderErrorBoundary resetKey={`${pathname}${search}`} fullScreen={fullScreen}>
    <Suspense fallback={<RouteLoadingFallback fullScreen={fullScreen} />}>
      {children}
    </Suspense>
    </RenderErrorBoundary>
  );
}
