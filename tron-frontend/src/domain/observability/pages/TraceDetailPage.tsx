import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { triggerDetail } from "../components/traceTrigger";
import { Link, useParams, useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useWorkspaceSpaces } from "@/shared";
import { PlatformWorkspaceFilter } from "@/domain/settings/components/PlatformWorkspaceScope";
import { fetchModelSelectOptions } from "@/domain/model/api";
import { useAuthStore } from "@/store/authStore";
import { useTraceDetail } from "../hooks/useObservability";
import { TraceInspector } from "../components/TraceInspector";

export function TraceDetailPage() {
  const { t } = useTranslation("observability");
  const { traceId = "" } = useParams();
  const [params] = useSearchParams();
  const [showFull, setShowFull] = useState(false);
  const rootSpanId = params.get("rootSpanId");
  const backParams = new URLSearchParams(params);
  backParams.delete("rootSpanId");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspaceCode =
    (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const detail = useTraceDetail(workspaceCode, traceId);
  const userCode = useAuthStore((state) => state.session?.user.userCode ?? "");
  const modelOptions = useQuery({
    queryKey: ["observability-model-options", userCode, workspaceCode],
    queryFn: ({ signal }) => fetchModelSelectOptions(workspaceCode, signal),
    enabled: Boolean(userCode && detail.data?.spans.some((span) => span.attributes["gen_ai.request.model"])),
    staleTime: 30_000,
    retry: false,
  });
  return (
    <>
      <header className="mb-3 flex flex-wrap items-center justify-between gap-3" aria-label={t("detail")}>
        <PlatformWorkspaceFilter />
        <div className="ml-auto flex flex-wrap items-center gap-3">
            {rootSpanId && <Button variant="outline" onClick={() => setShowFull((value) => !value)}>
              {t(showFull ? "showTriggerTrace" : "showFullTrace")}
            </Button>}
            <Button
              variant="outline"
              nativeButton={false}
              render={
                <Link
                  to={`/settings/observability/traces${backParams.size ? `?${backParams}` : ""}`}
                />
              }
            >
              {t("backToTraces")}
            </Button>
            <Button
              variant="outline"
              disabled={detail.isFetching}
              onClick={() => void detail.refetch()}
            >
              {detail.isFetching ? t("loading") : t("refresh")}
            </Button>
        </div>
      </header>
      {detail.isPending || detail.isFetching ? (
        <p role="status">{t("loading")}</p>
      ) : detail.isError ? (
        <div role="alert">
          {t("loadFailed")}
          <Button variant="secondary" onClick={() => void detail.refetch()}>
            {t("retry")}
          </Button>
        </div>
      ) : detail.data ? (
        <TraceInspector
          key={`${workspaceCode}:${traceId}:${showFull}:${rootSpanId}`}
          detail={showFull ? detail.data : triggerDetail(detail.data, rootSpanId)}
          models={modelOptions.data ?? []}
        />
      ) : null}
    </>
  );
}
