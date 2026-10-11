import { LogContextDialog } from "../components/LogContextDialog";
import { LogHighlight } from "../components/LogHighlight";
import { useToast } from "@/components/ui/toast";
import { PlatformWorkspaceFilter } from "@/domain/settings/components/PlatformWorkspaceScope";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SurfaceCard, useLocaleFormatters } from "@/shared";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ContinuousLogs } from "../components/ContinuousLogs";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectItem,
  SelectPopup,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DataTable } from "@/components/data-table";
import { TablePagination } from "@/components/table-pagination";
import { useApplicationLogs } from "../hooks/useObservability";
import { TraceIdentifier } from "../components/TraceIdentifier";
import { LogMessage } from "../components/LogMessage";
import { useUserTimeZone } from "@/shared/hooks/useUserTimeZone";
import { dateInTimeZone } from "@/shared/lib/userTimeZone";
import type { ApplicationLog, LogLevel } from "../model/ApplicationLog";
const emptyFilters = {
  keyword: "",
  level: "" as LogLevel | "",
  traceId: "",
  spanId: "",
};
export function LogsPage() {
  const { t } = useTranslation("observability");
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const workspaceCode = useWorkspaceSpaces().selectedPrimarySpace?.workspaceCode ?? "";
  const { timeZone } = useUserTimeZone();
  const [selectedDate, setSelectedDate] = useState("");
  const [clock, setClock] = useState(() => new Date());
  useEffect(() => {
    const timer = setInterval(() => setClock(new Date()), 60000);
    return () => clearInterval(timer);
  }, []);
  const today = dateInTimeZone(timeZone, clock);
  const date = selectedDate || today;
  const isToday = date === today;
  const [draft, setDraft] = useState(emptyFilters);
  const [filters, setFilters] = useState(emptyFilters);
  const [page, setPage] = useState(1);
  const [size, setSize] = useState(15);
  const [view, setView] = useState("continuous");
  const [invalid, setInvalid] = useState(false);
  const [contextLog, setContextLog] = useState<ApplicationLog | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  useEffect(() => {
    setPage(1);
    setExpanded(null);
  }, [date, timeZone]);
  const query = useApplicationLogs(
    { ...filters, workspaceCode, date, timeZone, current: page, size },
    isToday && view === "table",
    view === "table"
  );
  const total = query.data?.total ?? 0;
  const validId = (id: string, length: number) =>
    !id ||
    (new RegExp(`^[a-f0-9]{${length}}$`, "i").test(id) && !/^0+$/.test(id));
  const search = (values: typeof draft) => {
    const next = {
      ...values,
      keyword: values.keyword.trim(),
      traceId: values.traceId.trim().toLowerCase(),
      spanId: values.spanId.trim().toLowerCase(),
    };
    if (!validId(next.traceId, 32) || !validId(next.spanId, 16)) {
      setInvalid(true);
      toast.error(t("invalidLogIds"));
      return;
    }
    setInvalid(false);
    setDraft(next);
    setFilters(next);
    setPage(1);
    setExpanded(null);
    if (page === 1 && JSON.stringify(filters) === JSON.stringify(next))
      void query.refetch();
  };
  const identifier = (row: ApplicationLog, field: "traceId" | "spanId") =>
    row[field] ? (
      <TraceIdentifier
        className={field === "traceId" ? "w-32" : "w-28"}
        value={row[field]}
        label={field === "traceId" ? "Trace ID" : t("spanId")}
        onClick={() => search({ ...filters, [field]: row[field]! })}
      />
    ) : (
      <span className="text-[var(--color-on-surface-variant)]">—</span>
    );
  const filterToolbar = (
    <form
      className="grid w-full min-w-0 grid-cols-1 items-center gap-3 sm:grid-cols-2 lg:grid-cols-3"
      onSubmit={(e) => {
        e.preventDefault();
        search(draft);
      }}
    >
      <div className="min-w-0 [&>div]:w-full [&_button]:!w-full"><PlatformWorkspaceFilter /></div>
      <Input
        className="w-full min-w-0 font-mono"
        aria-label="Trace ID"
        placeholder="Trace ID"
        value={draft.traceId}
        aria-invalid={invalid && !validId(draft.traceId.trim(), 32)}
        onChange={(e) => setDraft({ ...draft, traceId: e.target.value })}
      />
      <Input
        className="w-full min-w-0 font-mono"
        aria-label={t("spanId")}
        placeholder={t("spanId")}
        value={draft.spanId}
        aria-invalid={invalid && !validId(draft.spanId.trim(), 16)}
        onChange={(e) => setDraft({ ...draft, spanId: e.target.value })}
      />
      <Input
        className="w-full min-w-0"
        aria-label={t("logKeyword")}
        placeholder={t("logKeyword")}
        maxLength={256}
        value={draft.keyword}
        onChange={(e) => setDraft({ ...draft, keyword: e.target.value })}
      />
      <Select
        value={draft.level || "All"}
        onValueChange={(value) =>
          setDraft({
            ...draft,
            level: value === "All" ? "" : (value as LogLevel),
          })
        }
      >
        <SelectTrigger className="w-full min-w-0" aria-label={t("logLevel")}>
          <SelectValue>
            {draft.level ? draft.level.toUpperCase() : t("allLogLevels")}
          </SelectValue>
        </SelectTrigger>
        <SelectPopup>
          <SelectItem value="All">{t("allLogLevels")}</SelectItem>
          {(["Error", "Warn", "Info", "Debug", "Trace"] as const).map(
            (level) => (
              <SelectItem key={level} value={level}>
                {level.toUpperCase()}
              </SelectItem>
            )
          )}
        </SelectPopup>
      </Select>
      <Input
        type="date"
        title={timeZone}
        className="w-full min-w-0"
        aria-label={t("logDate")}
        value={date}
        onChange={(e) => {
          setSelectedDate(e.target.value);
          setPage(1);
          setExpanded(null);
        }}
      />
      <div className="flex gap-2 sm:col-span-2 lg:col-span-3">
        <Button type="submit">
          {t("search")}
        </Button>
        <Button
          type="button"
          variant="outline"
          onClick={() => search(emptyFilters)}
        >
          {t("resetFilters")}
        </Button>
      </div>

    </form>
  );
  return (
    <>
      <Tabs className="h-full min-h-0" value={view} onValueChange={(value) => setView(String(value))}>
        <TabsList className="shrink-0" aria-label={t("logView")}>
          <TabsTrigger value="continuous">{t("continuousLogs")}</TabsTrigger>
          <TabsTrigger value="table">{t("logBlocks")}</TabsTrigger>
        </TabsList>
        <TabsContent value="continuous" className="flex min-h-0 flex-1 flex-col">
          <SurfaceCard className="mb-4 shrink-0">{filterToolbar}</SurfaceCard>
          {view === "continuous" && <ContinuousLogs key={JSON.stringify({ filters, date, timeZone })} filters={{ ...filters, workspaceCode, date, timeZone }} live isToday={isToday} onFilter={(field, value) => search({ ...filters, [field]: value })} />}
        </TabsContent>
        <TabsContent value="table">
      <DataTable<ApplicationLog>
        toolbar={
          <div className="w-full min-w-0 space-y-4">
            <h2 className="text-base font-semibold">{t("applicationLogs")}</h2>
            {filterToolbar}
          </div>
        }
        tableClassName="min-w-[960px] table-fixed"
        rows={query.data?.records ?? []}
        rowKey={(row) => row.logId}
        onRowClick={setContextLog}
        isRowExpanded={(row) => expanded === row.logId}
        renderExpandedRow={(row) => (
          <div className="space-y-3 py-2">
            <h3 className="font-medium">{t("logAttributes")}</h3>
            <dl className="grid grid-cols-[minmax(6rem,auto)_1fr] gap-x-6 gap-y-2 text-xs">
              {Object.entries(row.attributes).map(([key, value]) => (
                <div key={key} className="contents">
                  <dt className="font-mono">{key}</dt>
                  <dd className="whitespace-pre-wrap break-all font-mono">
                    {value}
                  </dd>
                </div>
              ))}
            </dl>
            {!Object.keys(row.attributes).length && <span>—</span>}
          </div>
        )}
        emptyState={
          <span role="status">{query.isPending || query.isError ? null : t("noLogs")}</span>
        }
        columns={[
          {
            id: "time",
            header: t("logTime"),
            width: "9rem",
            render: (row) =>
              formatDateTime(row.timestamp, {
                dateStyle: "short",
                timeStyle: "medium",
              }),
          },
          {
            id: "level",
            header: t("logLevel"),
            width: "5rem",
            render: (row) => (
              <Badge
                variant={
                  row.level === "Error"
                    ? "error"
                    : row.level === "Warn"
                    ? "warning"
                    : row.level === "Info"
                    ? "info"
                    : "secondary"
                }
              >
                {row.level.toUpperCase()}
              </Badge>
            ),
          },
          {
            id: "message",
            header: t("logMessage"),
            cellClassName: "whitespace-normal",
            render: (row) => (
              <div className="min-w-0 space-y-1">
                <LogMessage message={row.message} keyword={filters.keyword} />
                <p
                  className="truncate font-mono text-xs text-[var(--color-on-surface-variant)]"
                  title={row.target}
                >
                  <LogHighlight text={row.target} keyword={filters.keyword} />
                </p>
              </div>
            ),
          },
          {
            id: "trace",
            header: "Trace ID",
            width: "11rem",
            render: (row) => identifier(row, "traceId"),
          },
          {
            id: "span",
            header: t("spanId"),
            width: "10rem",
            render: (row) => identifier(row, "spanId"),
          },
          {
            id: "details",
            header: t("logDetails"),
            width: "7rem",
            render: (row) => (
              <Button
                variant="ghost"
                size="sm"
                aria-expanded={expanded === row.logId}
                onClick={() =>
                  setExpanded(expanded === row.logId ? null : row.logId)
                }
              >
                {t("logDetails")}
              </Button>
            ),
          },
        ]}
        pagination={
          <TablePagination
            totalItems={total}
            startItem={total ? (page - 1) * size + 1 : 0}
            endItem={Math.min(page * size, total)}
            page={page}
            pageCount={Math.max(1, Math.ceil(total / size))}
            pageSize={size}
            onPageChange={setPage}
            onPageSizeChange={(value) => {
              setSize(value);
              setPage(1);
            }}
            itemLabel={t("logs")}
          />
        }
      />
        </TabsContent>
      </Tabs>
      {contextLog && <LogContextDialog key={`${contextLog.logId}:${workspaceCode}:${date}:${timeZone}`} anchor={contextLog} scope={{ workspaceCode, date, timeZone }} onClose={() => setContextLog(null)} />}
    </>
  );
}
