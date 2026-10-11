import { useEffect, useState } from "react";
import { Package } from "lucide-react";
import { Link, useParams } from "react-router";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
import { PageHeader, SurfaceCard } from "@/shared";
import { listCapabilityPacks, type CapabilityPackRecord } from "../api/capabilityApi";

export function CapabilityPackDetailPage() {
  const { packId } = useParams<{ packId: string }>();
  const { t } = useTranslation(["shell", "pages"]);
  const [record, setRecord] = useState<CapabilityPackRecord>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [revision, setRevision] = useState(0);
  const pageT = (key: string) => t(`pages:capabilityManagement.${key}`);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError(undefined);
    setRecord(undefined);
    listCapabilityPacks(controller.signal).then((records) => {
      if (!controller.signal.aborted) setRecord(records.find((pack) => pack.id === packId));
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : String(reason));
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [packId, revision]);

  const groups = new Map<string, NonNullable<CapabilityPackRecord["contents"]>>();
  for (const item of record?.contents ?? []) {
    const group = groups.get(item.kind) ?? [];
    group.push(item);
    groups.set(item.kind, group);
  }

  return <div data-theme-part="capability-pack-detail-page" className="space-y-5">
    <PageHeader title={record?.name ?? pageT("details")} breadcrumbs={[
      { label: t("shell:settings.items.capabilityPacks"), href: "/settings/capability-packs" },
      { label: record?.name ?? pageT("details") },
    ]} actions={<div className="flex gap-2">
      <Link className={buttonVariants({ variant: "outline", size: "sm" })} to="/settings/capability-packs">{pageT("backToPacks")}</Link>
      <Button variant="outline" size="sm" disabled={loading} onClick={() => setRevision((value) => value + 1)}>{pageT("refresh")}</Button>
    </div>} />
    {loading ? <p role="status" className="text-sm text-[var(--color-on-surface-variant)]">{pageT("loading")}</p> : error ? <SurfaceCard role="alert" className="text-sm text-[var(--color-error)]">{pageT("loadFailed")}: {error}</SurfaceCard> : !record ? <SurfaceCard>{pageT("notFound")}</SurfaceCard> : <>
      <SurfaceCard className="space-y-4">
        <div className="flex items-start gap-3"><Package size={24} className="shrink-0" /><div className="min-w-0 flex-1"><h1 className="text-lg font-semibold">{record.name}</h1><p className="mt-1 break-all font-mono text-xs text-[var(--color-on-surface-variant)]">{record.id}</p></div><Badge variant={record.effective ? "success" : "outline"}>{pageT(record.effective ? "effective" : "inactive")}</Badge></div>
        <p className="text-sm text-[var(--color-on-surface-variant)]">{record.description}</p>
        <dl className="grid grid-cols-1 gap-3 border-t border-[var(--color-outline-variant)] pt-3 text-sm sm:grid-cols-3">
          {([["version", record.version], ["provider", record.provider], ["updatedAt", record.updatedAt ?? "—"]] as const).map(([key, value]) => <div key={key}><dt className="text-[var(--color-on-surface-variant)]">{pageT(key)}</dt><dd className="mt-1 break-words font-medium">{value}</dd></div>)}
        </dl>
      </SurfaceCard>
      <h2 className="text-base font-semibold">{pageT("contents")} <span className="font-mono text-sm text-[var(--color-on-surface-variant)]">{record.contents?.length ?? 0}</span></h2>
      {groups.size === 0 ? <SurfaceCard className="text-sm text-[var(--color-on-surface-variant)]">{pageT("emptyContents")}</SurfaceCard> : Array.from(groups, ([kind, items]) => <SurfaceCard key={kind} className="space-y-3">
        <h3 className="text-sm font-semibold">{t(`pages:capabilityManagement.contentKinds.${kind}`, { defaultValue: kind })} <span className="font-mono text-[var(--color-on-surface-variant)]">{items.length}</span></h3>
        <ul className="divide-y divide-[var(--color-outline-variant)]">{items.map((item, index) => <li key={`${item.name}-${index}`} className="py-3 first:pt-0 last:pb-0"><p className="break-all font-mono text-sm font-medium">{item.name}</p>{item.description ? <p className="mt-1 whitespace-pre-wrap break-words text-sm text-[var(--color-on-surface-variant)]">{item.description}</p> : null}</li>)}</ul>
      </SurfaceCard>)}
    </>}
  </div>;
}
