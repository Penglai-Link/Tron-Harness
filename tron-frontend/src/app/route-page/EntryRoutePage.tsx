import { fetchBootstrapProfileStatus, type BootstrapProfileStatus } from "@/domain/profile/api";
import { useState, useEffect } from "react";
import { Navigate } from "react-router";
import RouteLoadingFallback from "./RouteLoadingFallback";

export default function EntryRoutePage() {
  const [profileStatus, setProfileStatus] = useState<BootstrapProfileStatus | null>(null);
  const [profileError, setProfileError] = useState<string | null>(null);

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
          setProfileError(
            error instanceof Error ? error.message : "无法读取 PersonAI 档案状态",
          );
      });
    return () => {
      active = false;
    };
  }, []);

  if (profileError) {
    return (
      <main data-theme-anchor="app-shell" className="flex min-h-screen items-center justify-center bg-[var(--color-background)] px-6 text-center">
        <div>
          <h1 className="text-xl font-semibold">无法读取 PersonAI 档案状态</h1>
          <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">
            {profileError}
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

  return <Navigate replace to="/workspace" />;
}
