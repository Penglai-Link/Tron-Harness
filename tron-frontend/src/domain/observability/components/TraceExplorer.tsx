import { appRoutes } from "@/router/routes";
import { InfoTooltip } from "@/components/info-tooltip/InfoTooltip";
import { SelectItem } from "@/components/ui/select";
import { triggerLabel } from "./traceTrigger";
import { PlatformWorkspaceFilter } from "@/domain/settings/components/PlatformWorkspaceScope";
import { useEffect, useRef, useState } from "react";
import { useSearchParams, useNavigate } from "react-router";
import { useTranslation } from "react-i18next";

import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { TextInput, SelectInput } from "@/components/forms";
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
  const { t, i18n } = useTranslation("observability");
  const { currentLocale, formatDateTime, formatNumber } = useLocaleFormatters();
  const [params, setParams] = useSearchParams();
  const category = params.get("category") === "Agent" ? "Agent" : "Page";
  const sessionCode = params.get("sessionCode") ?? "";
  const traceId = params.get("traceId") ?? "";
  const spanId = params.get("spanId") ?? "";
  const keyword = params.get("keyword") ?? "";
  const status = params.get("status") ?? "";
  const agentCode = params.get("agentCode") ?? "";
  const [keywordInput, setKeywordInput] = useState(keyword);
  const setFilter = (key: string, value: string) => {
    const next = new URLSearchParams(params);
    if (value) next.set(key, value); else next.delete(key);
    setCurrent(1);
    setParams(next);
  };
  const keywordAliases = keyword.trim() ? [...new Set(appRoutes
    .filter((route) => triggerLabel(route.route, i18n).toLowerCase().includes(keyword.trim().toLowerCase()))
    .flatMap((route) => [route.route.split("/:")[0], route.title]))] : [];
  const navigate = useNavigate();
  const [sessionInput, setSessionInput] = useState(sessionCode);
  const [spanInput, setSpanInput] = useState(spanId);
  const [traceInput, setTraceInput] = useState(traceId);
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const toast = useToast();
  const notifiedError = useRef<string | null>(null);
  const [sorts, setSorts] = useState<{ field: TraceSortField; direction: "Asc" | "Desc" }[]>([{ field: "StartedAt", direction: "Desc" }]);
  const list = useTraceList({
    sorts,
    keyword, status: status || null, agentCode: category === "Agent" ? agentCode || null : null, keywordAliases,
    category,
    workspaceCode,
    sessionCode,
    traceId,
    spanId: spanId || null,
    current,
    size,
  });
  useEffect(() => {
    setSessionInput(sessionCode);
    setKeywordInput(keyword);
    setSpanInput(spanId);
    setTraceInput(traceId);
    setCurrent(1);
  }, [sessionCode, traceId, spanId, keyword, status, agentCode, category]);
  useEffect(() => {
    if (list.isSuccess) notifiedError.current = null;
    if (list.error && notifiedError.current !== list.error.message) {
      notifiedError.current = list.error.message;
      toast.error(list.error.message);
    }
  }, [list.error, list.isSuccess, toast]);
  const page = list.data;
  const total = page?.total ?? 0;
  useEffect(() => {
    if (list.isSuccess && !list.isPlaceholderData) {
      setCurrent((value) => Math.min(value, Math.max(1, Math.ceil(total / size))));
    }
  }, [list.isSuccess, list.isPlaceholderData, total, size]);
  const select = (row: TraceSummary) => {
    const next = new URLSearchParams(params);
    if (row.rootSpanId) next.set("rootSpanId", row.rootSpanId);
    navigate(`/settings/observability/traces/${encodeURIComponent(row.traceId)}?${next}`);
  };
  const filterSession = (code: string) => {
    if (!code) return;
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
    const span = spanInput.trim().toLowerCase();
    if (span && (!/^[a-f0-9]{16}$/.test(span) || /^0+$/.test(span))) {
      toast.error(t("invalidSpanId"));
      return;
    }
    const next = new URLSearchParams(params);
    next.delete("sessionCode"); next.delete("spanId"); next.delete("traceId"); next.delete("keyword");
    if (sessionInput.trim()) next.set("sessionCode", sessionInput.trim());
    if (keywordInput.trim()) next.set("keyword", keywordInput.trim());
    if (span) next.set("spanId", span);
    if (id) next.set("traceId", id);
    if (current === 1 && sessionCode === sessionInput.trim() && spanId === span && traceId === id && keyword === keywordInput.trim()) void list.refetch();
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
      <TextInput aria-label={t("keyword")} placeholder={t("keywordPlaceholder")} className="w-48 max-w-full" value={keywordInput} maxLength={256} onChange={(event) => setKeywordInput(event.target.value)} />
      <TextInput
        aria-label={t("sessionId")}
        placeholder={t("sessionId")}
        className="w-48 max-w-full font-mono"
        value={sessionInput}
        onChange={(event) => setSessionInput(event.target.value)}
      />
      <TextInput
        aria-label="Trace ID"
        placeholder="Trace ID"
        className="h-[var(--tron-input-height,2.25rem)] w-48 max-w-full font-mono sm:h-[var(--tron-input-height,2.25rem)]"
        value={traceInput}
        onChange={(e) => setTraceInput(e.target.value)}
        maxLength={32}
      />
      <TextInput
        aria-label="Span ID"
        placeholder="Span ID"
        className="h-[var(--tron-input-height,2.25rem)] w-48 max-w-full sm:h-[var(--tron-input-height,2.25rem)]"
        value={spanInput}
        onChange={(e) => setSpanInput(e.target.value)}
        maxLength={16}
      />
      <SelectInput aria-label={t("state")} className="w-36 max-w-full" value={status || "__all__"} onValueChange={(value) => setFilter("status", value === "__all__" ? "" : String(value))}>
        <SelectItem value="__all__">{t("allStates")}</SelectItem>
        {["Running", "Completed", "Error", "Cancelled", "Unknown", "Waiting", "Interrupted"].map((state) => <SelectItem key={state} value={state}>{t(`status.${state}`)}</SelectItem>)}
      </SelectInput>
      {category === "Agent" && <SelectInput aria-label={t("agent")} className="w-48 max-w-full" value={agentCode || "__all__"} onValueChange={(value) => setFilter("agentCode", value === "__all__" ? "" : String(value))}>
        <SelectItem value="__all__">{t("allAgents")}</SelectItem>
        {agentCode && !list.data?.agentOptions?.some((option) => option.code === agentCode) && <SelectItem value={agentCode}>{agentCode}</SelectItem>}
        {(list.data?.agentOptions ?? []).map((option) => <SelectItem key={option.code} value={option.code}>{option.name}</SelectItem>)}
      </SelectInput>}
      <Button type="submit" >
        {t("search")}
      </Button>
      {(sessionCode || traceId || spanId || keyword || status || agentCode) && (
        <Button
          variant="ghost"
          type="button"
          onClick={() => {
            const next = new URLSearchParams(params); for (const key of ["sessionCode", "traceId", "spanId", "keyword", "status", "agentCode"]) next.delete(key); setParams(next);
            setKeywordInput("");
            setCurrent(1);
            setSpanInput("");
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
    <Tabs value={category} onValueChange={(value) => {
      const next = new URLSearchParams(params);
      next.set("category", String(value));
      setCurrent(1);
      setParams(next);
    }}>
      <TabsList variant="line" aria-label={t("traces")}>
        <TabsTrigger value="Page">{t("pageTraces")}</TabsTrigger>
        <TabsTrigger value="Agent">{t("agentTraces")}</TabsTrigger>
      </TabsList>
      <TabsContent value={category} aria-busy={list.isFetching}>
      <DataTable<TraceSummary>
        title={t("traces")}
        toolbar={toolbar}
        rows={page?.records ?? []}
        rowKey={(row) => `${row.traceId}:${row.rootSpanId ?? ""}`}
        emptyState={emptyState}
        pagination={pagination}
        columns={[
          { id: "trigger", header: <span className="inline-flex items-center gap-1">{t("trigger")}<InfoTooltip ariaLabel={t("triggerHelpLabel")}>{t("triggerHelp")}</InfoTooltip></span>, width: "200px", render: (row) => (
            <Button variant="link" className="h-auto max-w-48 justify-start truncate p-0" onClick={() => select(row)}>
              {triggerLabel(row.trigger || row.agentCode, i18n, category)}
            </Button>
          ) },
          { id: "workspace", header: t("workspace"), width: "160px", render: (row) => <span className="block max-w-36 truncate" title={row.workspaceCode}>{row.workspaceCode || "—"}</span> },
          {
            id: "trace",
            header: "Trace ID",
            width: "208px",
            render: (row) => (
              <TraceIdentifier
                value={row.traceId}
                label="Trace ID"
                onClick={() => select(row)}
              />
            ),
          },
          {
            id: "session",
            header: <span className="inline-flex items-center gap-1">{t("sessionCode")}<InfoTooltip ariaLabel={t("sessionHelpLabel")}>{t("sessionHelp")}</InfoTooltip></span>,
            width: "208px",
            render: (row) => row.sessionCode ? (
              <TraceIdentifier
                value={row.sessionCode}
                description={row.sessionName || t("sessionUnavailable")}
                label={t("sessionId")}
                onClick={() => filterSession(row.sessionCode)}
              />
            ) : "—",
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
    </TabsContent>
    </Tabs>
  );
}
