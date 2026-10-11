import { PlatformWorkspaceFilter } from "@/domain/settings/components/PlatformWorkspaceScope";
import { useEffect, useRef, useState } from "react";
import { useSearchParams, useNavigate } from "react-router";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { TextInput } from "@/components/forms";
import { DataTable } from "@/components/data-table";
import { TablePagination } from "@/components/table-pagination";
import { useToast } from "@/components/ui/toast";
import { useLocaleFormatters } from "@/shared";
import { useTraceList } from "../hooks/useObservability";
import { TraceStatus } from "./TraceStatus";
import { TraceIdentifier } from "./TraceIdentifier";
import { formatTraceDuration, formatTraceTokens } from "./traceFormat";
import type { TraceSortField, TraceSummary } from "../model/Trace";

export function TraceExplorer({ workspaceCode }: { workspaceCode: string }) {
  const { t } = useTranslation("observability");
  const { currentLocale, formatDateTime, formatNumber } = useLocaleFormatters();
  const [params, setParams] = useSearchParams();
  const sessionCode = params.get("sessionCode") ?? "";
  const traceId = params.get("traceId") ?? "";
  const navigate = useNavigate();
  const [sessionInput, setSessionInput] = useState(sessionCode);
  const [traceInput, setTraceInput] = useState(traceId);
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const toast = useToast();
  const notifiedError = useRef<string | null>(null);
  const [sorts, setSorts] = useState<{ field: TraceSortField; direction: "Asc" | "Desc" }[]>([{ field: "StartedAt", direction: "Desc" }]);
  const list = useTraceList({
    sorts,
    workspaceCode,
    sessionCode,
    traceId,
    current,
    size,
  });
  useEffect(() => {
    setSessionInput(sessionCode);
    setTraceInput(traceId);
    setCurrent(1);
  }, [sessionCode, traceId]);
  useEffect(() => {
    if (list.isSuccess) notifiedError.current = null;
    if (list.error && notifiedError.current !== list.error.message) {
      notifiedError.current = list.error.message;
      toast.error(list.error.message);
    }
  }, [list.error, list.isSuccess, toast]);
  const page = list.data;
  const total = page?.total ?? 0;
  const select = (id: string) =>
    navigate(
      `/settings/observability/traces/${encodeURIComponent(id)}${
        params.size ? `?${params}` : ""
      }`
    );
  const filterSession = (code: string) => {
    const next = new URLSearchParams(params);
    next.set("sessionCode", code);
    setParams(next);
  };
  const search = (event: React.FormEvent) => {
    event.preventDefault();
    const id = traceInput.trim().toLowerCase();
    if (id && (!/^[a-f0-9]{32}$/.test(id) || /^0+$/.test(id))) {
      toast.error(t("invalidTraceId"));
      return;
    }
    const next = new URLSearchParams(params);
    next.delete("sessionCode"); next.delete("traceId");
    if (sessionInput.trim()) next.set("sessionCode", sessionInput.trim());
    if (id) next.set("traceId", id);
    if (current === 1 && sessionCode === sessionInput.trim() && traceId === id) void list.refetch();
    setParams(next);
    setCurrent(1);
  };
  const sortAria = (field: TraceSortField) => {
    const sort = sorts.find((item) => item.field === field);
    return sort ? (sort.direction === "Asc" ? "ascending" as const : "descending" as const) : "none" as const;
  };
  const sortableHeader = (label: string, field: TraceSortField) => {
    const index = sorts.findIndex((item) => item.field === field);
    const sort = sorts[index];
    return <Button variant="ghost" size="sm" className="h-auto px-0 font-medium" onClick={() => {
      setSorts((previous) => {
        const existing = previous.find((item) => item.field === field);
        if (!existing) return [...previous, { field, direction: "Desc" }];
        return existing.direction === "Desc"
          ? previous.map((item) => item.field === field ? { ...item, direction: "Asc" } : item)
          : previous.filter((item) => item.field !== field);
      });
      setCurrent(1);
    }}>
      {label}{sort ? ` ${sort.direction === "Asc" ? "↑" : "↓"}` : ""}
    </Button>;
  };
  const toolbar = (
    <form
      className="flex min-w-0 flex-wrap items-center gap-3"
      onSubmit={search}
    >
      <PlatformWorkspaceFilter />
      <TextInput
        aria-label="Trace ID"
        placeholder="Trace ID"
        className="h-[var(--tron-input-height,2.25rem)] w-48 max-w-full font-mono sm:h-[var(--tron-input-height,2.25rem)]"
        value={traceInput}
        onChange={(e) => setTraceInput(e.target.value)}
        maxLength={128}
      />
      <TextInput
        aria-label={t("sessionCode")}
        placeholder={t("sessionCode")}
        className="h-[var(--tron-input-height,2.25rem)] w-48 max-w-full sm:h-[var(--tron-input-height,2.25rem)]"
        value={sessionInput}
        onChange={(e) => setSessionInput(e.target.value)}
        maxLength={256}
      />
      <Button type="submit" >
        {t("search")}
      </Button>
      {(sessionCode || traceId) && (
        <Button
          variant="ghost"
          type="button"
          onClick={() => {
            const next = new URLSearchParams(params); next.delete("sessionCode"); next.delete("traceId"); setParams(next);
            setSessionInput("");
            setTraceInput("");
          }}
        >
          {t("reset")}
        </Button>
      )}
    </form>
  );
  const emptyState = (
    <span
      role="status"
      className="block w-fit max-w-[calc(100vw-6rem)] text-left"
    >
      {list.isPending || list.isError ? null : t("noTraces")}
    </span>
  );
  const pagination = (
    <TablePagination
      totalItems={total}
      startItem={total ? (current - 1) * size + 1 : 0}
      endItem={total ? (current - 1) * size + (page?.records.length ?? 0) : 0}
      page={current}
      pageCount={Math.max(1, Math.ceil(total / size))}
      pageSize={size}
      onPageChange={(value) =>
        setCurrent(
          Math.max(1, Math.min(value, Math.max(1, Math.ceil(total / size))))
        )
      }
      onPageSizeChange={(value) => {
        setSize(value);
        setCurrent(1);
      }}
      itemLabel={t("traces")}
    />
  );
  return (
    <>
      <DataTable<TraceSummary>
        title={t("traces")}
        toolbar={toolbar}
        rows={page?.records ?? []}
        rowKey={(row) => row.traceId}
        emptyState={emptyState}
        pagination={pagination}
        columns={[
          { id: "workspace", header: t("workspace"), width: "160px", render: (row) => <span className="block max-w-36 truncate" title={row.workspaceCode}>{row.workspaceCode || "—"}</span> },
          {
            id: "trace",
            header: "Trace ID",
            width: "208px",
            render: (row) => (
              <TraceIdentifier
                value={row.traceId}
                label="Trace ID"
                onClick={() => select(row.traceId)}
              />
            ),
          },
          {
            id: "session",
            header: t("sessionCode"),
            width: "208px",
            render: (row) => (
              <TraceIdentifier
                value={row.sessionCode}
                description={row.sessionName || t("sessionUnavailable")}
                label={t("sessionId")}
                onClick={() => filterSession(row.sessionCode)}
              />
            ),
          },
          {
            id: "agent",
            header: t("agent"),
            render: (row) => row.agentCode,
          },
          {
            id: "status",
            header: t("state"),
            render: (row) => <TraceStatus status={row.status} />,
          },
          {
            id: "start",
            ariaSort: sortAria("StartedAt"),
            header: sortableHeader(t("startedAt"), "StartedAt"),
            render: (row) => (
              <span className="whitespace-nowrap tabular-nums">
                {formatDateTime(row.startedAt, {
                  dateStyle: "short",
                  timeStyle: "medium",
                })}
              </span>
            ),
          },
          {
            id: "duration",
            ariaSort: sortAria("Duration"),
            header: sortableHeader(t("duration"), "Duration"),
            align: "right",
            render: (row) => (
              <span className="tabular-nums">
                {formatTraceDuration(row.durationMs, currentLocale)}
              </span>
            ),
          },
          {
            id: "spans",
            ariaSort: sortAria("Spans"),
            header: sortableHeader("Spans", "Spans"),
            align: "right",
            render: (row) => formatNumber(row.spanCount),
          },
          {
            id: "tokens",
            ariaSort: sortAria("Tokens"),
            header: sortableHeader("Tokens", "Tokens"),
            align: "right",
            render: (row) =>
              formatTraceTokens(
                row.inputTokens + row.outputTokens,
                currentLocale
              ),
          },
          {
            id: "errors",
            ariaSort: sortAria("Errors"),
            header: sortableHeader(t("errors"), "Errors"),
            align: "right",
            render: (row) => (
              <span
                className={
                  row.errorCount ? "text-[var(--color-error)]" : undefined
                }
              >
                {formatNumber(row.errorCount)}
              </span>
            ),
          },
        ]}
      />
    </>
  );
}
