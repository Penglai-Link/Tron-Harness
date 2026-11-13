import { useEffect, useState } from "react";
import { Package } from "lucide-react";
import { useParams } from "react-router";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { MarkdownContent } from "@/components/markdown-content/MarkdownContent";
import { capabilityDescription, capabilityPackDetails } from "../model/capabilityDescription";
import { PageHeader, SurfaceCard } from "@/shared";
import { getCapabilityPack, type CapabilityPackRecord } from "../api/capabilityApi";

const CONTENT_KIND_ORDER = [
  "Tool", "Skill", "Resource", "Page", "Navigation", "Mcp", "Runtime", "Context", "Policy",
  "Extension", "Implementation", "Plugin", "Dependency", "Permission",
] as const;

const NAVIGATION_KINDS = new Set(["NavigationDomain", "NavigationMenu", "NavigationLayout"]);

export function CapabilityPackDetailPage() {
  const { packId } = useParams<{ packId: string }>();
  const { t, i18n } = useTranslation(["shell", "pages"]);
  const [record, setRecord] = useState<CapabilityPackRecord>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const pageT = (key: string) => t(`pages:capabilityManagement.${key}`);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError(undefined);
    setRecord(undefined);
    getCapabilityPack(packId ?? "", controller.signal).then((record) => {
      if (!controller.signal.aborted) setRecord(record);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : String(reason));
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [packId, i18n.resolvedLanguage]);

  const groups = new Map<string, NonNullable<CapabilityPackRecord["contents"]>>();
  for (const item of record?.contents ?? []) {
    const kind = NAVIGATION_KINDS.has(item.kind) ? "Navigation" : item.kind;
    const group = groups.get(kind) ?? [];
    group.push(item);
    groups.set(kind, group);
  }
  const kinds = [
    ...CONTENT_KIND_ORDER,
    ...Array.from(groups.keys()).filter((kind) => !CONTENT_KIND_ORDER.some((known) => known === kind)).sort(),
  ];
  const contentGroups = kinds.map((kind) => ({ kind, items: groups.get(kind) ?? [] })).filter(({ items }) => items.length > 0);
  const defaultKind = contentGroups.find(({ items }) => items.length > 0)?.kind ?? CONTENT_KIND_ORDER[0];
  const readme = record?.readme?.trim();

  return <div data-theme-part="capability-pack-detail-page" className="space-y-4">
    <PageHeader className="mb-0" title={record?.name ?? pageT("details")} breadcrumbs={[
      { label: t("shell:settings.items.capabilityPacks"), href: "/settings/capability-packs" },
      { label: record?.name ?? pageT("details") },
    ]} />
    {loading ? <p role="status" className="text-sm text-[var(--color-on-surface-variant)]">{pageT("loading")}</p> : error ? <SurfaceCard role="alert" className="text-sm text-[var(--color-error)]">{pageT("loadFailed")}: {error}</SurfaceCard> : !record ? <SurfaceCard>{pageT("notFound")}</SurfaceCard> : <div className="flex min-w-0 flex-col gap-4">
      <SurfaceCard className="space-y-3 p-4">
        <div className="flex items-start gap-3"><Package size={24} className="shrink-0" /><div className="min-w-0 flex-1"><h1 className="text-lg font-semibold">{record.name}</h1><p className="mt-1 break-all font-mono text-xs text-[var(--color-on-surface-variant)]">{record.id}</p></div><Badge variant={record.effective ? "success" : "outline"}>{pageT(record.effective ? "effective" : "inactive")}</Badge></div>
        <MarkdownContent className="max-w-4xl text-sm leading-6 text-[var(--color-on-surface-variant)]" content={capabilityPackDetails(t, record.id, record.description)} />
        <dl className={`grid grid-cols-1 gap-3 border-t border-[var(--color-outline-variant)] pt-3 text-sm ${record.id.startsWith("tron.computer-use") ? "sm:grid-cols-2 xl:grid-cols-4" : "sm:grid-cols-3"}`}>
          <div><dt className="text-[var(--color-on-surface-variant)]">{pageT("packVersion")}</dt><dd className="mt-1 break-words font-medium">{record.version}</dd></div>
          {record.id.startsWith("tron.computer-use") && <div><dt className="text-[var(--color-on-surface-variant)]">{pageT("cuaDriverSdkVersion")}</dt><dd className="mt-1 break-words font-medium">{record.cuaDriverSdkVersion ?? pageT("versionUndeclared")}</dd></div>}
          {([["provider", record.provider], ["updatedAt", record.updatedAt ?? "—"]] as const).map(([key, value]) => <div key={key}><dt className="text-[var(--color-on-surface-variant)]">{pageT(key)}</dt><dd className="mt-1 break-words font-medium">{value}</dd></div>)}
        </dl>
      </SurfaceCard>
      <Tabs key={record.id} defaultValue={readme ? "readme" : defaultKind} className="min-w-0">
        <TabsList aria-label={pageT("contents")} className="max-w-full flex-wrap justify-start gap-1 box-border! group-data-horizontal/tabs:h-auto">
          {readme && <TabsTrigger value="readme" className="flex-none px-2 2xl:px-3">README</TabsTrigger>}
          {contentGroups.map(({ kind, items }) => <TabsTrigger key={kind} value={kind} className="flex-none gap-2 px-2 2xl:px-3">
            {t(`pages:capabilityManagement.contentKinds.${kind}`, { defaultValue: kind })}
            <span className="font-mono text-xs text-[var(--color-on-surface-variant)]">{items.length}</span>
          </TabsTrigger>)}
        </TabsList>
        {readme && <TabsContent value="readme">
          <SurfaceCard className="min-w-0 p-4 sm:p-6">
            <MarkdownContent ariaLabel="README" content={readme} />
          </SurfaceCard>
        </TabsContent>}
        {contentGroups.map(({ kind, items }) => <TabsContent key={kind} value={kind}>
          {items.length === 0 ? <SurfaceCard className="text-sm text-[var(--color-on-surface-variant)]">{pageT("emptyContents")}</SurfaceCard> :
            <ul className="grid gap-3 sm:grid-cols-2">{items.map((item, index) => {
              return <li key={`${item.name}-${index}`}>
                <SurfaceCard className="p-3">
                  {kind === "Navigation" && <Badge variant="outline" className="mb-2">{t(`pages:capabilityManagement.contentKinds.${item.kind}`, { defaultValue: item.kind })}</Badge>}
                  <h2 className="break-words text-sm font-semibold">{item.displayName || item.name}</h2>
                  {item.displayName && item.displayName !== item.name && <p className="mt-1 break-all font-mono text-xs text-[var(--color-on-surface-variant)]">{item.name}</p>}
                  {item.kind !== "Plugin" && (item.kind !== "NavigationLayout" || item.description) ? <p className="mt-2 max-w-4xl whitespace-pre-wrap break-words text-sm text-[var(--color-on-surface-variant)]">{(item.kind === "Tool" ? item.description : capabilityDescription(t, item.kind, item.name, item.description)) || pageT("noDescription")}</p> : null}
                  {item.kind === "NavigationLayout" && !!item.provides?.length && <ul className="mt-2 space-y-1 text-xs text-[var(--color-on-surface-variant)]">{item.provides.map((menuCode, index) => <li key={`${menuCode}-${index}`} className="break-all font-mono">{menuCode}</li>)}</ul>}
                </SurfaceCard>
              </li>;
            })}</ul>}
        </TabsContent>)}
      </Tabs>

    </div>}
  </div>;
}
