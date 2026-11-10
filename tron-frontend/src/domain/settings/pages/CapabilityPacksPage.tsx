import { capabilityDescription } from "../model/capabilityDescription";
import { useEffect, useMemo, useRef, useState } from "react";
import { CheckCircle2, Package } from "lucide-react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { SelectInput } from "@/components/forms";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { SelectItem } from "@/components/ui/select";
import { PageHeader, SurfaceCard } from "@/shared";
import { notifyCapabilityResourcesChanged } from "@/domain/skill/api";
import {
  uninstallCapabilityPack,
  importCapabilityPack,
  listCapabilityPacks,
  setCapabilityPackEnabled,
  type CapabilityPackRecord,
} from "../api/capabilityApi";

export function CapabilityPacksPage() {
  const { t } = useTranslation(["shell", "pages"]);
  const [records, setRecords] = useState<CapabilityPackRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const toast = useToast();
  const [loadFailed, setLoadFailed] = useState(false);
  const [provider, setProvider] = useState("all");
  const [status, setStatus] = useState("effective");
  const [busyId, setBusyId] = useState<string | null>(null);
  const importInputRef = useRef<HTMLInputElement>(null);
  const [pendingImport, setPendingImport] = useState<File | null>(null);
  const load = async (signal?: AbortSignal) => {
    setLoading(true);
    setLoadFailed(false);
    try {
      setRecords(await listCapabilityPacks(signal));
    } catch (reason) {
      if (!signal?.aborted) {
        setLoadFailed(true);
        toast.error(reason instanceof Error ? reason.message : String(reason), {
          title: t("pages:capabilityManagement.loadFailed"),
        });
      }
    } finally {
      if (!signal?.aborted) setLoading(false);
    }
  };
  useEffect(() => {
    const controller = new AbortController();
    void load(controller.signal);
    return () => controller.abort();
  }, []);
  const pageT = (key: string) => t(`pages:capabilityManagement.${key}`);
  const providers = useMemo(
    () => Array.from(new Set(records.map((record) => record.provider))).sort(),
    [records]
  );
  const visibleRecords = records.filter(
    (record) =>
      record.kind === "capability" &&
      (provider === "all" || record.provider === provider) &&
      (status === "all" || record.effective === (status === "effective"))
  );
  const importPack = async (file?: File) => {
    if (!file || busyId !== null) return;
    setPendingImport(null);
    setBusyId("import");
    try {
      await importCapabilityPack(file);
      await load();
      notifyCapabilityResourcesChanged();
      toast.success(pageT("importSucceeded"));
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : String(reason), { title: pageT("importFailed") });
    } finally {
      setBusyId(null);
    }
  };
  const togglePack = async (record: CapabilityPackRecord) => {
    setBusyId(record.id);
    try {
      await setCapabilityPackEnabled(record.id, !record.effective);
      toast.success(pageT(record.effective ? "disableSucceeded" : "enableSucceeded"));
      await load();
      notifyCapabilityResourcesChanged();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : String(reason), { title: pageT(record.effective ? "disableFailed" : "enableFailed") });
    } finally {
      setBusyId(null);
    }
  };

  const removePack = async (record: CapabilityPackRecord) => {
    setBusyId(record.id);
    try {
      await uninstallCapabilityPack(record.id);
      toast.success(pageT("uninstallSucceeded"));
      await load();
      notifyCapabilityResourcesChanged();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e), { title: pageT("uninstallFailed") });
    } finally {
      setBusyId(null);
    }
  };
  return (
    <div
      data-theme-part="capability-packs-page"
      className="space-y-5 [--tron-capability-effective-fg:var(--color-success)] [--tron-capability-inactive-fg:var(--color-on-surface-variant)] [--tron-capability-id-fg:var(--color-on-surface-variant)]"
    >
      <ConfirmDialog
        open={pendingImport !== null}
        title={pageT("import")}
        description={`${pendingImport?.name ?? ""} — ${pageT("developmentImportWarning")}`}
        tone="default"
        cancelText={pageT("cancelImport")}
        confirmText={pageT("confirmImport")}
        onCancel={() => setPendingImport(null)}
        onConfirm={() => void importPack(pendingImport ?? undefined)}
      />
      <PageHeader
        title={t("shell:settings.items.capabilityPacks")}
        actions={
          <div className="flex flex-wrap gap-2">
            <input
              ref={importInputRef}
              aria-label={pageT("import")}
              className="sr-only"
              type="file"
              accept=".zip,application/zip"
              disabled={busyId !== null}
              onChange={(event) => {
                setPendingImport(event.currentTarget.files?.[0] ?? null);
                event.currentTarget.value = "";
              }}
            />
            <Button
              variant="outline"
              size="sm"
              disabled={busyId !== null}
              onClick={() => importInputRef.current?.click()}
            >

              {pageT("import")}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void load()}
              disabled={loading}
            >

              {pageT("refresh")}
            </Button>
          </div>
        }
      />
      <div className="flex w-full max-w-xl flex-wrap items-center gap-3">
        <SelectInput
          className="h-9 min-w-40 flex-1"
          aria-label={pageT("filterProvider")}
          value={provider}
          onValueChange={(value) => setProvider(String(value ?? "all"))}
        >
          <SelectItem value="all">{pageT("allProviders")}</SelectItem>
          {providers.map((name) => (
            <SelectItem key={name} value={name}>
              {name}
            </SelectItem>
          ))}
        </SelectInput>
        <SelectInput
          className="h-9 w-36"
          aria-label={pageT("filterStatus")}
          value={status}
          onValueChange={(value) => setStatus(String(value ?? "effective"))}
        >
          <SelectItem value="effective">{pageT("effective")}</SelectItem>
          <SelectItem value="inactive">{pageT("inactive")}</SelectItem>
          <SelectItem value="all">{pageT("allStatuses")}</SelectItem>
        </SelectInput>
      </div>
      {loading && records.length === 0 ? (
        <p className="text-sm text-[var(--color-on-surface-variant)]">
          {pageT("loading")}
        </p>
      ) : null}
      {!loading && !loadFailed && visibleRecords.length === 0 ? (
        <SurfaceCard className="text-sm text-[var(--color-on-surface-variant)]">
          {pageT(records.some((record) => record.kind === "capability") ? "noMatchingPacks" : "emptyPacks")}
        </SurfaceCard>
      ) : null}
      <div className="grid gap-3 xl:grid-cols-2">
        {visibleRecords.map((record) => (
          <SurfaceCard
            data-theme-part="capability-pack-card"
            data-settings-card
            key={record.id}
            className="relative space-y-4 p-4 transition-colors hover:border-[var(--color-outline)]"
          >
            <div className="flex items-start justify-between gap-3">
              <div className="flex min-w-0 items-start gap-3">
                <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-[var(--color-surface-container)]">
                  <Package size={18} />
                </span>
                <div className="min-w-0">
                  <h2 className="truncate text-sm font-semibold">
                    <Link
                      className="after:absolute after:inset-0 after:rounded-lg focus-visible:outline focus-visible:outline-2"
                      to={`/settings/capability-packs/${encodeURIComponent(
                        record.id
                      )}`}
                    >
                      {record.name}
                    </Link>
                  </h2>
                  <p
                    data-theme-part="capability-pack-id"
                    className="mt-1 break-all font-mono text-xs text-[var(--tron-capability-id-fg,var(--color-on-surface-variant))]"
                  >
                    {record.id}
                  </p>
                </div>
              </div>
              <Badge
                data-theme-part={
                  record.effective
                    ? "capability-status-effective"
                    : "capability-status-inactive"
                }
                variant={record.effective ? "success" : "outline"}
              >
                {record.effective ? (
                  <>
                    <CheckCircle2 size={13} className="mr-1" />
                    {pageT("effective")}
                  </>
                ) : (
                  pageT("inactive")
                )}
              </Badge>
            </div>
            <p className="min-h-10 text-sm text-[var(--color-on-surface-variant)]">
              {capabilityDescription(t, "Pack", record.id, record.description)}
            </p>
            <dl className="grid grid-cols-3 gap-3 border-t border-[var(--color-outline-variant)] pt-3 text-xs">
              <div>
                <dt className="text-[var(--color-on-surface-variant)]">
                  {pageT("version")}
                </dt>
                <dd className="mt-1 font-medium">{record.version}</dd>
              </div>
              <div>
                <dt className="text-[var(--color-on-surface-variant)]">
                  {pageT("provider")}
                </dt>
                <dd className="mt-1 font-medium">{record.provider}</dd>
              </div>
              <div>
                <dt className="text-[var(--color-on-surface-variant)]">
                  {pageT("updatedAt")}
                </dt>
                <dd className="mt-1 font-medium">{record.updatedAt ?? "—"}</dd>
              </div>
            </dl>
            <div className="flex justify-end gap-2 border-t border-[var(--color-outline-variant)] pt-3">
              {record.provider === "External" ? (
                <Button
                  className="relative z-10"
                  variant="outline"
                  size="sm"
                  disabled={busyId !== null}
                  onClick={() => void removePack(record)}
                >
                  {t("pages:agentSkills.uninstallPack")}
                </Button>
              ) : null}
              <Button
                className="relative z-10"
                size="sm"
                variant={record.effective ? "outline" : "default"}
                disabled={busyId !== null}
                onClick={() => void togglePack(record)}
              >
                {busyId === record.id
                  ? pageT("saving")
                  : record.effective
                  ? pageT("disable")
                  : pageT("enable")}
              </Button>
            </div>
          </SurfaceCard>
        ))}
      </div>
    </div>
  );
}
