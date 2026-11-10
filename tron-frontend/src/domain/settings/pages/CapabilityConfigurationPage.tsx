import { useEffect, useState } from "react";
import { CheckCircle2, CircleDashed } from "lucide-react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";

import { DataTable, type DataTableColumn } from "@/components/data-table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { SurfaceCard } from "@/shared";
import { capabilityDescription } from "../model/capabilityDescription";
import { ToolPolicySettings } from "../components/ToolPolicySettings";
import { listExtensionImplementations, setExtensionEnabled, type ExtensionImplementationRecord } from "../api/capabilityApi";

const EXTENSION_POINT_ORDER = [
  "model", "tool", "tool-policy", "skill", "observe", "context", "runtime", "external-agent", "storage",
] as const;

export function CapabilityConfigurationPage() {
  const { t, i18n } = useTranslation(["shell", "pages"]);
  const [records, setRecords] = useState<ExtensionImplementationRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError(null);
    listExtensionImplementations(controller.signal).then((items) => {
      if (!controller.signal.aborted) setRecords(items);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : String(reason));
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [revision, i18n.resolvedLanguage]);

  const pageT = (key: string) => t(`pages:capabilityManagement.${key}`);
  const label = (point: string) => t(`pages:capabilityManagement.extensionPoints.${point}`, {
    defaultValue: records.find((record) => record.extensionPoint === point)?.extensionName ?? point,
  });
  const points = [
    ...EXTENSION_POINT_ORDER,
    ...Array.from(new Set(records.map((record) => record.extensionPoint)))
      .filter((point) => !EXTENSION_POINT_ORDER.some((known) => known === point)).sort(),
  ];
  const groups = points.map((point) => ({ point, items: records.filter((record) => record.extensionPoint === point) }));
  const toggle = async (record: ExtensionImplementationRecord) => {
    setBusyId(record.extensionPoint);
    setError(null);
    try {
      setRecords(await setExtensionEnabled(record.extensionPoint, !record.enabled));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusyId(null);
    }
  };
  const columns: DataTableColumn<ExtensionImplementationRecord>[] = [
    { id: "point", header: pageT("extensionPoint"), width: "12%", render: (record) => <div><p className="font-medium">{label(record.extensionPoint)}</p><p className="mt-1 break-all font-mono text-xs text-[var(--color-on-surface-variant)]">{record.extensionPoint}</p></div> },
    { id: "implementation", header: pageT("currentImplementation"), width: "20%", cellClassName: "whitespace-normal break-words", render: (record) => <span className="font-medium">{record.implementation}</span> },
    { id: "description", header: pageT("implementationDescription"), width: "30%", cellClassName: "whitespace-normal", render: (record) => <span className="text-sm text-[var(--color-on-surface-variant)]">{capabilityDescription(t, "Implementation", record.implementation, record.description) || pageT("noDescription")}</span> },
    { id: "provider", header: pageT("providedBy"), width: "20%", render: (record) => <div><p>{record.provider}</p><p className="mt-1 break-all font-mono text-xs text-[var(--color-on-surface-variant)]">{record.providerPackId}</p></div> },
    { id: "status", header: pageT("status"), width: "10%", render: (record) => <Badge variant={record.effective ? "success" : "outline"}>{record.effective ? <><CheckCircle2 size={13} />{pageT("effective")}</> : <><CircleDashed size={13} />{pageT("inactive")}</>}</Badge> },
    { id: "actions", header: pageT("configuration"), align: "right", width: "12%", render: (record) => {
      if (record.configuration === "Required") return <span className="text-xs text-[var(--color-on-surface-variant)]">{pageT("requiredService")}</span>;
      if (record.configuration === "CapabilityPack") return <Button size="sm" variant="outline" nativeButton={false} render={<Link to={`/settings/capability-packs/${encodeURIComponent(record.providerPackId)}`} />}>{pageT("configurePack")}</Button>;
      return <Button size="sm" variant={record.enabled ? "outline" : "default"} disabled={busyId !== null || loading} aria-label={`${pageT(record.enabled ? "disable" : "enable")} ${label(record.extensionPoint)}`} onClick={() => void toggle(record)}>{busyId === record.extensionPoint ? pageT("saving") : `${pageT(record.enabled ? "disable" : "enable")} ${label(record.extensionPoint)}`}</Button>;
    } },
  ];

  return <div data-theme-part="capability-configuration-page" className="space-y-5 [--tron-capability-effective-fg:var(--color-success)] [--tron-capability-inactive-fg:var(--color-on-surface-variant)] [--tron-capability-provider-fg:var(--color-on-surface-variant)]">
    {error ? <SurfaceCard role="alert" className="text-sm text-[var(--color-error)]">{pageT("loadFailed")}: {error}</SurfaceCard> : null}
    {loading && records.length === 0 ? <p role="status" className="text-sm text-[var(--color-on-surface-variant)]">{pageT("loading")}</p> : null}
    <Tabs defaultValue="model" className="min-w-0">
      <div className="flex flex-wrap items-center justify-between gap-3">
      {!loading || records.length > 0 ? <TabsList aria-label={pageT("extensionPoint")} data-theme-part="capability-extension-tabs" className="max-w-full flex-wrap justify-start gap-1 box-border! bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)] group-data-horizontal/tabs:h-auto">
        {groups.map(({ point, items }) => <TabsTrigger key={point} value={point} className="flex-none gap-2 px-3 text-[var(--color-on-surface-variant)] data-active:bg-[var(--color-surface)] data-active:text-[var(--color-on-surface)] dark:data-active:bg-[var(--color-surface-container-high)]">
          {label(point)}<span className="font-mono text-xs">{items.length}</span>
        </TabsTrigger>)}
      </TabsList> : null}
      <Button className="ml-auto" size="sm" variant="outline" disabled={loading || busyId !== null} onClick={() => setRevision((value) => value + 1)}>{pageT("refresh")}</Button>
      </div>
      {!loading || records.length > 0 ? groups.map(({ point, items }) => <TabsContent key={point} value={point} className="min-w-0">
        {point === "tool-policy" ? <ToolPolicySettings revision={revision} onSavingChange={(saving) => setBusyId(saving ? "tool-policy" : null)} /> : null}
        <DataTable className="overflow-x-auto" rows={items}
          rowKey={(row) => `${row.extensionPoint}:${row.providerPackId}:${row.implementation}`}
          pagination={null} emptyState={pageT("emptyExtensions")} columns={columns} tableClassName="min-w-[960px]" />
      </TabsContent>) : null}
    </Tabs>
  </div>;
}
