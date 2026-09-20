import { fetchBootstrapProfileStatus, type BootstrapProfileStatus } from "@/domain/profile/api";
import { useState, useEffect } from "react";
import { Navigate } from "react-router";
import RouteLoadingFallback from "./RouteLoadingFallback";

export default function EntryRoutePage() {
  const [profileStatus, setProfileStatus] = useState<BootstrapProfileStatus | null>(null);
  const [setupError, setSetupError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    fetchBootstrapProfileStatus()
      .then((next) => {
        if (!active) return;
        setProfileStatus(next);
        return null;
      })
      .catch((error: unknown) => {
        if (active)
          setSetupError(
            error instanceof Error ? error.message : "无法读取初始化状态",
          );
      });
    return () => {
      active = false;
    };
  }, []);

  if (setupError) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-[var(--color-background)] px-6 text-center">
        <div>
          <h1 className="text-xl font-semibold">无法读取初始化状态</h1>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
            {setupError}
          </p>
        </div>
      </main>
    );
  }

  if (!profileStatus) {
    return <RouteLoadingFallback fullScreen />;
  }

  if (!profileStatus.hasProfiles || !profileStatus.activeProfile || profileStatus.activeProfile.status === "PENDING") {
    return <Navigate replace to="/profiles" />;
  }

  return <Navigate replace to="/home" />;
}
