import { SetupStatus, fetchSetupStatus } from "@/domain/setup/api";
import { useState, useEffect } from "react";
import { Navigate } from "react-router";
import RouteLoadingFallback from "./RouteLoadingFallback";
import LoginRoutePage from "./LoginRoutePage";

export default function EntryRoutePage() {
  const [setupStatus, setSetupStatus] = useState<SetupStatus | null>(null);
  const [setupError, setSetupError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    fetchSetupStatus()
      .then((next) => {
        if (active) setSetupStatus(next);
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

  if (!setupStatus) {
    return <RouteLoadingFallback fullScreen />;
  }

  if (!setupStatus.initialized) {
    return <Navigate replace to="/setup" />;
  }

  return <LoginRoutePage />;
}
