import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import { PageHeader } from "@/shared";
import { DataTable } from "@/components/data-table";
import { TablePagination } from "@/components/table-pagination";
import { SearchField } from "@/components/search-field";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { useToast } from "@/components/ui/toast";
import { usePlatformDataNavigate } from "@/domain/settings/components/PlatformWorkspaceScope";
import { useAuthSession } from "@/domain/auth";
import { useMemoryTargets } from "../hooks";
import {
  fetchMemoryRecords,
  updateMemoryRecord,
  deleteMemoryRecord,
} from "../api";
import { memoryScopes } from "../constants";
import { MemoryRecordSummary } from "../components";
import type { MemoryRecord, MemoryScope } from "../model";
export function MemoryPage() {
  const { t, i18n } = useTranslation("pages");
  const navigate = usePlatformDataNavigate();
  const toast = useToast();
  const cache = useQueryClient();
  const { session } = useAuthSession();
  const { targets, domains } = useMemoryTargets();
  const [params, setParams] = useSearchParams();
  const scope = (memoryScopes as readonly string[]).includes(
    params.get("scope") ?? "",
  )
    ? (params.get("scope") as MemoryScope)
    : "User";
  const available = targets.filter((v) => v.scope === scope);
  const target =
    available.find((v) => v.scopeCode === params.get("scopeCode")) ??
    available[0];
  const [keyword, setKeyword] = useState("");
  const [status, setStatus] = useState("All");
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(20);
  const [deleting, setDeleting] = useState<MemoryRecord | null>(null);
  const [busy, setBusy] = useState(false);
  const query = useQuery({
    queryKey: [
      "memory-records",
      session?.user.userCode,
      target?.scope,
      target?.scopeCode,
      keyword,
      status,
      current,
      size,
    ],
    queryFn: ({ signal }) =>
      fetchMemoryRecords(
        {
          scope,
          scopeCode: target!.scopeCode,
          keyword,
          enabled: status === "All" ? null : status === "Enabled",
          current,
          size,
        },
        signal,
      ),
    enabled: !!target?.scopeCode,
  });
  const data = query.data;
  const total = data?.total ?? 0;
  function changeScope(scope: MemoryScope, scopeCode?: string) {
    const next = new URLSearchParams(params);
    next.set("scope", scope);
    if (scopeCode) next.set("scopeCode", scopeCode);
    else next.delete("scopeCode");
    setParams(next);
    setCurrent(1);
  }
  function open(r: MemoryRecord, edit = false) {
    navigate(
      `/memory/details?memoryId=${encodeURIComponent(r.memoryCode)}${edit ? "&mode=edit" : ""}`,
    );
  }
  async function toggle(r: MemoryRecord) {
    if (busy) return;
    setBusy(true);
    try {
      await updateMemoryRecord(r.memoryCode, {
        version: r.version,
        enabled: !r.enabled,
      });
      await cache.invalidateQueries({ queryKey: ["memory-records"] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : t("memory.simple.failed"));
    } finally {
      setBusy(false);
    }
  }
  async function remove() {
    if (!deleting || busy) return;
    setBusy(true);
    try {
      await deleteMemoryRecord(deleting.memoryCode, deleting.version);
      setDeleting(null);
      if ((data?.records.length ?? 0) === 1 && current > 1)
        setCurrent(current - 1);
      await cache.invalidateQueries({ queryKey: ["memory-records"] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : t("memory.simple.failed"));
    } finally {
      setBusy(false);
    }
  }
  const empty =
    domains.isError && scope === "WorkDomain" ? (
      <p role="alert">
        {domains.error.message}
        <Button variant="outline" onClick={() => void domains.refetch()}>
          {t("memory.simple.retry")}
        </Button>
      </p>
    ) : !target ? (
      <p role="status">
        {t(
          scope === "Space"
            ? "memory.simple.selectWorkspace"
            : "memory.simple.noDomains",
        )}
      </p>
    ) : query.isLoading ? (
      <p role="status">{t("memory.simple.loading")}</p>
    ) : query.isError ? (
      <p role="alert">
        {query.error.message}
        <Button variant="outline" onClick={() => void query.refetch()}>
          {t("memory.simple.retry")}
        </Button>
      </p>
    ) : (
      t("memory.simple.empty")
    );
  return (
    <main className="space-y-5">
      <PageHeader
        title={t("memory.title")}
        actions={
          <Button
            disabled={!target?.scopeCode}
            onClick={() =>
              navigate(
                `/memory/new?scope=${scope}&scopeCode=${encodeURIComponent(target!.scopeCode)}`,
              )
            }
          >
            {t("memory.simple.new")}
          </Button>
        }
      />
      <Tabs
        value={scope}
        onValueChange={(v) => changeScope(String(v) as MemoryScope)}
      >
        <TabsList>
          {memoryScopes.map((v) => (
            <TabsTrigger key={v} value={v}>
              {t(`memory.simple.scope.${v}`)}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>
      <DataTable<MemoryRecord>
        rows={query.isFetching ? [] : (data?.records ?? [])}
        rowKey={(r) => r.memoryCode}
        tableClassName="min-w-[900px]"
        emptyState={
          query.isFetching ? (
            <p role="status">{t("memory.simple.loading")}</p>
          ) : (
            empty
          )
        }
        toolbar={
          <>
            <SearchField
              ariaLabel={t("memory.simple.search")}
              placeholder={t("memory.simple.search")}
              value={keyword}
              onChange={(v) => {
                setKeyword(v);
                setCurrent(1);
              }}
            />
            <SelectInput
              aria-label={t("memory.simple.status")}
              value={status}
              onValueChange={(v) => {
                setStatus(String(v));
                setCurrent(1);
              }}
            >
              {["All", "Enabled", "Disabled"].map((v) => (
                <SelectItem key={v} value={v}>
                  {t(`memory.simple.statusValue.${v}`)}
                </SelectItem>
              ))}
            </SelectInput>
            {scope === "WorkDomain" && available.length > 0 ? (
              <SelectInput
                aria-label={t("memory.simple.domain")}
                value={target?.scopeCode}
                onValueChange={(v) => changeScope(scope, String(v))}
              >
                {available.map((v) => (
                  <SelectItem key={v.scopeCode} value={v.scopeCode}>
                    {v.label}
                  </SelectItem>
                ))}
              </SelectInput>
            ) : null}
            <Button
              variant="outline"
              disabled={query.isFetching || !target}
              onClick={() => void query.refetch()}
            >
              {t("memory.simple.refresh")}
            </Button>
          </>
        }
        columns={[
          {
            id: "memory",
            header: t("memory.simple.content"),
            width: "38%",
            render: (r) => (
              <MemoryRecordSummary memory={r} onOpen={() => open(r)} />
            ),
          },
          {
            id: "scope",
            header: t("memory.simple.ownership"),
            render: (r) =>
              targets.find(
                (v) => v.scope === r.scope && v.scopeCode === r.scopeCode,
              )?.label ?? r.scopeCode,
          },
          {
            id: "enabled",
            header: t("memory.simple.status"),
            render: (r) => (
              <Badge variant={r.enabled ? "success" : "info"}>
                {t(
                  `memory.simple.statusValue.${r.enabled ? "Enabled" : "Disabled"}`,
                )}
              </Badge>
            ),
          },
          {
            id: "source",
            header: t("memory.simple.source"),
            render: (r) =>
              t(`memory.simple.sourceValue.${r.sourceLabel}`, r.sourceLabel),
          },
          {
            id: "updated",
            header: t("memory.simple.updated"),
            render: (r) =>
              new Intl.DateTimeFormat(i18n.language, {
                dateStyle: "short",
                timeStyle: "short",
              }).format(r.updatedAt),
          },
          {
            id: "actions",
            header: t("memory.simple.actions"),
            render: (r) => (
              <div className="flex flex-wrap gap-1">
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => open(r, true)}
                >
                  {t("memory.simple.edit")}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => void toggle(r)}
                >
                  {t(
                    r.enabled
                      ? "memory.simple.disable"
                      : "memory.simple.enable",
                  )}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => setDeleting(r)}
                >
                  {t("memory.simple.delete")}
                </Button>
              </div>
            ),
          },
        ]}
        pagination={
          <TablePagination
            page={current}
            pageSize={size}
            pageSizeOptions={[20, 50, 100]}
            pageCount={Math.max(1, Math.ceil(total / size))}
            totalItems={total}
            startItem={total ? (current - 1) * size + 1 : 0}
            endItem={Math.min(current * size, total)}
            onPageChange={setCurrent}
            onPageSizeChange={(v) => {
              setSize(v);
              setCurrent(1);
            }}
          />
        }
      />
      <ConfirmDialog
        open={!!deleting}
        pending={busy}
        title={t("memory.simple.deleteConfirm")}
        description={deleting?.title}
        confirmText={t("memory.simple.delete")}
        cancelText={t("memory.simple.cancel")}
        onCancel={() => !busy && setDeleting(null)}
        onConfirm={() => void remove()}
      />
    </main>
  );
}
