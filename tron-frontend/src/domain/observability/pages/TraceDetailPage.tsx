import { useState } from "react";
import { triggerDetail } from "../components/traceTrigger";
import { Link, useParams, useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useWorkspaceSpaces } from "@/shared";
import { PlatformWorkspaceFilter } from "@/domain/settings/components/PlatformWorkspaceScope";
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
  return (
    <>
      <header className="mb-3 flex flex-wrap items-center justify-between gap-3" aria-label={t("detail")}>
        <PlatformWorkspaceFilter />
        <div className="ml-auto flex flex-wrap items-center gap-3">
            {rootSpanId && <Button variant="secondary" onClick={() => setShowFull((value) => !value)}>
              {t(showFull ? "showTriggerTrace" : "showFullTrace")}
            </Button>}
            <Button
              variant="secondary"
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
              variant="secondary"
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
        />
      ) : null}
    </>
  );
}
