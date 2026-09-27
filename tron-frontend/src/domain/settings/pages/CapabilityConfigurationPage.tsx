import { useEffect, useState } from "react";
import { CheckCircle2, CircleDashed } from "lucide-react";
import { useTranslation } from "react-i18next";

import { DataTable, type DataTableColumn } from "@/components/data-table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { PageHeader, SurfaceCard } from "@/shared";
import { listExtensionImplementations, setExtensionEnabled, type ExtensionImplementationRecord } from "../api/capabilityApi";

export function CapabilityConfigurationPage() {
  const { t } = useTranslation(["shell", "pages"]);
  const [records, setRecords] = useState<ExtensionImplementationRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    listExtensionImplementations(controller.signal).then(setRecords).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : String(reason));
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, []);
  const pageT = (key: string) => t(`pages:capabilityManagement.${key}`);
  const label = (record: ExtensionImplementationRecord) => t(`pages:capabilityManagement.extensionPoints.${record.extensionPoint}`, { defaultValue: record.extensionName });
  const columns: DataTableColumn<ExtensionImplementationRecord>[] = [
    { id: "point", header: pageT("extensionPoint"), width: "12%", render: (record) => <div><p className="font-medium">{label(record)}</p><p className="mt-1 font-mono text-xs text-[var(--color-on-surface-variant)]">{record.extensionPoint}</p></div> },
    { id: "implementation", header: pageT("currentImplementation"), width: "20%", render: (record) => <span className="font-medium">{record.implementation}</span> },
    { id: "description", header: pageT("implementationDescription"), width: "30%", cellClassName: "whitespace-normal", render: (record) => <span className="text-sm text-[var(--color-on-surface-variant)]">{record.description}</span> },
    { id: "provider", header: pageT("providedBy"), width: "20%", render: (record) => <div><p>{record.provider}</p><p className="mt-1 break-all font-mono text-xs text-[var(--color-on-surface-variant)]">{record.providerPackId}</p></div> },
    { id: "status", header: pageT("status"), width: "10%", render: (record) => <Badge variant={record.effective ? "success" : "outline"}>{record.effective ? <><CheckCircle2 size={13} />{pageT("effective")}</> : <><CircleDashed size={13} />{pageT("inactive")}</>}</Badge> },
    { id: "actions", header: pageT("actions"), align: "right", width: "8%", render: (record) => <Button size="sm" variant={record.effective ? "outline" : "default"} disabled={busyId !== null} onClick={async () => { setBusyId(record.extensionPoint); setError(null); try { setRecords(await setExtensionEnabled(record.extensionPoint, !record.effective)); } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); } finally { setBusyId(null); } }}>{busyId === record.extensionPoint ? pageT("saving") : record.effective ? pageT("disable") : pageT("enable")}</Button> },
  ];

  return <div data-theme-part="capability-configuration-page" className="space-y-5 [--tron-capability-effective-fg:var(--color-success)] [--tron-capability-inactive-fg:var(--color-on-surface-variant)] [--tron-capability-provider-fg:var(--color-on-surface-variant)]">
    <PageHeader title={t("shell:settings.items.capabilityConfiguration")} />
    {error ? <SurfaceCard role="alert" className="text-sm text-[var(--color-error)]">{pageT("loadFailed")}: {error}</SurfaceCard> : null}
    {loading && records.length === 0 ? <p className="text-sm text-[var(--color-on-surface-variant)]">{pageT("loading")}</p> : null}
    {!loading && !error && records.length === 0 ? <SurfaceCard className="text-sm text-[var(--color-on-surface-variant)]">{pageT("emptyExtensions")}</SurfaceCard> : null}
    {records.length > 0 ? <Tabs defaultValue={records[0].extensionPoint} className="min-w-0">
      <TabsList data-theme-part="capability-extension-tabs" className="max-w-full flex-wrap justify-start bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
        {records.map((record) => <TabsTrigger
          key={record.extensionPoint}
          value={record.extensionPoint}
          className="text-[var(--color-on-surface-variant)] data-active:bg-[var(--color-surface)] data-active:text-[var(--color-on-surface)] dark:data-active:bg-[var(--color-surface-container-high)]"
        >{label(record)}</TabsTrigger>)}
      </TabsList>
      {records.map((record) => <TabsContent key={record.extensionPoint} value={record.extensionPoint} className="min-w-0">
        <DataTable
          className="overflow-x-auto"
          rows={[record]}
          rowKey={(row) => `${row.extensionPoint}:${row.implementation}`}
          pagination={null}
          emptyState={pageT("emptyExtensions")}
          columns={columns}
          tableClassName="min-w-[960px]"
        />
      </TabsContent>)}
    </Tabs> : null}
  </div>;
}
