import { useEffect, useState, useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import { PageHeader, SurfaceCard } from "@/shared";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useToast } from "@/components/ui/toast";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { usePlatformDataNavigate } from "@/domain/settings/components/PlatformWorkspaceScope";
import { useAuthSession } from "@/domain/auth";
import { useMemoryTargets } from "../hooks";
import {
  createMemoryRecord,
  fetchMemoryRecord,
  updateMemoryRecord,
  deleteMemoryRecord,
} from "../api";
import { MemoryFormFields } from "./MemoryFormFields";
export function MemoryEditor({ create = false }: { create?: boolean }) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const navigate = usePlatformDataNavigate();
  const cache = useQueryClient();
  const [params] = useSearchParams();
  const code = params.get("memoryId") ?? "";
  const editing = create || params.get("mode") === "edit";
  const { session } = useAuthSession();
  const { targets, domains } = useMemoryTargets();
  const detail = useQuery({
    queryKey: ["memory-record", session?.user.userCode, code],
    queryFn: ({ signal }) => fetchMemoryRecord(code, signal),
    enabled: !create && !!code,
    refetchInterval: (query) =>
      query.state.data?.indexStatus === "Pending" ? 1000 : false,
  });
  const r = detail.data;
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [target, setTarget] = useState(
    () =>
      `${params.get("scope") ?? "User"}:${params.get("scopeCode") ?? session?.user.userCode ?? ""}`,
  );
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const initialized = useRef("");
  const editVersion = useRef<number | null>(null);
  useEffect(() => {
    if (
      r &&
      initialized.current !== `${r.memoryCode}:${r.version}` &&
      (!editing || !initialized.current.startsWith(`${r.memoryCode}:`))
    ) {
      setTitle(r.title);
      setText(r.text);
      setTarget(`${r.scope}:${r.scopeCode}`);
      initialized.current = `${r.memoryCode}:${r.version}`;
      editVersion.current = r.version;
    }
  }, [r, editing]);
  const selected = targets.find((v) => `${v.scope}:${v.scopeCode}` === target);
  const options =
    r && !selected
      ? [
          { scope: r.scope, scopeCode: r.scopeCode, label: r.scopeCode },
          ...targets,
        ]
      : targets;
  const back = () => navigate("/memory");
  function cancelEdit() {
    if (r) {
      setTitle(r.title);
      setText(r.text);
    }
    navigate(`/memory/details?memoryId=${encodeURIComponent(code)}`);
  }
  async function save() {
    if (busy || !text.trim() || (!r && !selected)) return;
    setBusy(true);
    try {
      const saved = create
        ? await createMemoryRecord({
            scope: selected!.scope,
            scopeCode: selected!.scopeCode,
            title,
            text,
          })
        : await updateMemoryRecord(code, { version: editVersion.current ?? r!.version, title, text });
      toast.success(
        t(
          saved.indexStatus === "Ready"
            ? "memory.simple.savedReady"
            : "memory.simple.savedPending",
        ),
      );
      cache.setQueryData(
        ["memory-record", session?.user.userCode, saved.memoryCode],
        saved,
      );
      await cache.invalidateQueries({ queryKey: ["memory-records"] });
      navigate(
        `/memory/details?memoryId=${encodeURIComponent(saved.memoryCode)}`,
        { replace: true },
      );
    } catch (e) {
      toast.error(e instanceof Error ? e.message : t("memory.simple.failed"));
    } finally {
      setBusy(false);
    }
  }
  async function toggle() {
    if (!r || busy) return;
    setBusy(true);
    try {
      const saved = await updateMemoryRecord(code, {
        version: r.version,
        enabled: !r.enabled,
      });
      cache.setQueryData(
        ["memory-record", session?.user.userCode, code],
        saved,
      );
      await cache.invalidateQueries({ queryKey: ["memory-records"] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : t("memory.simple.failed"));
    } finally {
      setBusy(false);
    }
  }
  async function remove() {
    if (!r || busy) return;
    setBusy(true);
    try {
      await deleteMemoryRecord(code, r.version);
      await cache.invalidateQueries({ queryKey: ["memory-records"] });
      back();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : t("memory.simple.failed"));
    } finally {
      setBusy(false);
    }
  }
  if (!create && (detail.isLoading || detail.isError || !r))
    return (
      <SurfaceCard>
        {detail.isLoading ? (
          <p role="status">{t("memory.simple.loading")}</p>
        ) : (
          <p role="alert">
            {detail.error?.message ?? t("memory.simple.empty")}
            <Button onClick={() => void detail.refetch()}>
              {t("memory.simple.retry")}
            </Button>
          </p>
        )}
      </SurfaceCard>
    );
  return (
    <main className="space-y-5">
      <PageHeader
        title={create ? t("memory.simple.new") : r!.title || t("memory.title")}
        actions={
          editing ? (
            <>
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => (create ? back() : cancelEdit())}
              >
                {t("memory.simple.cancel")}
              </Button>
              <Button
                disabled={busy || !text.trim() || (!r && !selected)}
                onClick={() => void save()}
              >
                {t(busy ? "memory.simple.saving" : "memory.simple.save")}
              </Button>
            </>
          ) : (
            <>
              <Button variant="outline" onClick={back}>
                {t("memory.simple.back")}
              </Button>
              <Button
                disabled={busy}
                onClick={() =>
                  navigate(
                    `/memory/details?memoryId=${encodeURIComponent(code)}&mode=edit`,
                  )
                }
              >
                {t("memory.simple.edit")}
              </Button>
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => void toggle()}
              >
                {t(
                  r!.enabled ? "memory.simple.disable" : "memory.simple.enable",
                )}
              </Button>
              <Button
                variant="ghost"
                disabled={busy}
                onClick={() => setConfirm(true)}
              >
                {t("memory.simple.delete")}
              </Button>
            </>
          )
        }
      />
      {domains.isError ? <p role="alert">{domains.error.message}</p> : null}
      {editing ? (
        <MemoryFormFields
          title={title}
          text={text}
          target={target}
          targets={options}
          locked={!create}
          disabled={busy}
          onTitle={setTitle}
          onText={setText}
          onTarget={setTarget}
        />
      ) : (
        <SurfaceCard className="space-y-5">
          <div className="flex gap-3">
            <Badge>
              {t(
                `memory.simple.statusValue.${r!.enabled ? "Enabled" : "Disabled"}`,
              )}
            </Badge>
            <span>{selected?.label ?? r!.scopeCode}</span>
          </div>
          <p className="whitespace-pre-wrap break-words leading-7">{r!.text}</p>
        </SurfaceCard>
      )}
      {r ? (
        <SurfaceCard className="space-y-3">
          <p>
            {t("memory.simple.source")}:{" "}
            {t(`memory.simple.sourceValue.${r.sourceLabel}`, r.sourceLabel)}
          </p>
          <pre className="max-h-80 overflow-auto whitespace-pre-wrap break-words text-xs">
            {JSON.stringify(r.sourceRefs, null, 2)}
          </pre>
          <p role="status">{t(`memory.simple.index.${r.indexStatus}`)}</p>
          {r.indexError ? (
            <p role="alert" className="break-words">
              {r.indexError}
            </p>
          ) : null}
        </SurfaceCard>
      ) : null}
      <ConfirmDialog
        open={confirm}
        pending={busy}
        title={t("memory.simple.deleteConfirm")}
        confirmText={t("memory.simple.delete")}
        cancelText={t("memory.simple.cancel")}
        onCancel={() => !busy && setConfirm(false)}
        onConfirm={() => void remove()}
      />
    </main>
  );
}
