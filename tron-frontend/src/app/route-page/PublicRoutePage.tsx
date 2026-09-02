import { routePages } from "@/router/pageRegistry";
import RouteLoadingFallback from "./RouteLoadingFallback";
import SuspendedRoute from "./SuspendedRoute";

export default function PublicRoutePage({ pageId }: { pageId: string }) {
  const Component = routePages[pageId];

  if (!Component) {
    return <RouteLoadingFallback fullScreen />;
  }

  return (
    <SuspendedRoute fullScreen>
      <Component />
    </SuspendedRoute>
  );
}
