import { ReactNode, Suspense } from "react";
import RouteLoadingFallback from "./RouteLoadingFallback";

export default function SuspendedRoute({
  children,
  fullScreen = false,
}: {
  children: ReactNode;
  fullScreen?: boolean;
}) {
  return (
    <Suspense fallback={<RouteLoadingFallback fullScreen={fullScreen} />}>
      {children}
    </Suspense>
  );
}
