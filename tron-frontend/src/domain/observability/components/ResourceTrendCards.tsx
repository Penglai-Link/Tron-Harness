import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { fetchPlatformSystemAgents, fetchWorkspaceInternalAgents, listTools } from "@/domain/agent/api/agentManagementApi";
import { PLATFORM_INTERNAL_AGENT_SCOPE } from "@/domain/agent/model";
import { listDudes } from "@/domain/dude/api/dudeApi";
import { CAPABILITY_RESOURCES_CHANGED, fetchRegistrySkills } from "@/domain/skill/api";
import { SurfaceCard, useLocaleFormatters } from "@/shared";
import { getCapabilityPluginCounts } from "@/domain/settings/api/capabilityApi";
import { queryMetricPoints } from "../api/observabilityApi";
import { useQueryErrorToast } from "../hooks/useObservability";
import type { MetricPoint } from "../model/Trace";

function samples(records: MetricPoint[]) {
  // Process charts describe the backend, independently of the workspace filter.
  const latest = records.reduce<MetricPoint | undefined>((a, b) => !a || b.timestamp > a.timestamp ? b : a, undefined);
  return records.filter(p => p.resource["service.instance.id"] === latest?.resource["service.instance.id"] && p.resource["process.pid"] === latest?.resource["process.pid"])
    .map(p => ({ time: p.timestamp, value: Number(p.data.asDouble ?? p.data.asInt) }))
    .filter(p => Number.isFinite(p.value)).sort((a, b) => a.time - b.time);
}

function Trend({ name, title, factor, unit, color }: { name: string; title: string; factor: number; unit: string; color: string }) {
  const { t } = useTranslation("observability");
  const { formatNumber, formatDateTime } = useLocaleFormatters();
  const query = useQuery({ queryKey: ["observability-resource-history", name], queryFn: ({ signal }) => queryMetricPoints(name, 1, 100, signal), refetchInterval: 10000 });
  useQueryErrorToast(query.error);
  const points = samples(query.data?.records ?? []).map(p => ({ ...p, value: p.value * factor }));
  const last = points.at(-1);
  const max = Math.max(1, ...points.map(p => p.value));
  const firstTime = points[0]?.time ?? 0;
  const duration = Math.max(1, (last?.time ?? 0) - firstTime);
  const path = points.map((p, i) => `${i ? "L" : "M"}${8 + (p.time - firstTime) / duration * 584},${142 - p.value / max * 128}`).join(" ");
  return <SurfaceCard className="!shadow-none">
    <div className="mb-4 flex items-center justify-between gap-3"><h2 className="font-semibold">{title}</h2><span className="text-sm tabular-nums">{last ? `${formatNumber(last.value, { maximumFractionDigits: 1 })} ${unit}` : "—"}</span></div>
    {points.length ? <>
      <svg viewBox="0 0 600 156" className="h-40 w-full" role="img" aria-label={title} preserveAspectRatio="none">
        {[14, 78, 142].map(y => <line key={y} x1="8" x2="592" y1={y} y2={y} stroke="var(--color-outline-variant)" strokeWidth="1" />)}
        <path d={`${path} L${8 + ((last?.time ?? firstTime) - firstTime) / duration * 584},142 L8,142 Z`} fill={color} opacity="0.1" />
        <path d={path} fill="none" stroke={color} strokeWidth="2.5" vectorEffect="non-scaling-stroke" />
        {points.map((p, i) => <circle key={`${p.time}-${i}`} cx={8 + (p.time - firstTime) / duration * 584} cy={142 - p.value / max * 128} r="2.5" fill={color}><title>{formatDateTime(p.time, { timeStyle: "medium" })} · {formatNumber(p.value, { maximumFractionDigits: 1 })} {unit}</title></circle>)}
      </svg>
      <div className="mt-2 flex justify-between text-xs text-[var(--color-on-surface-variant)]"><span>{formatDateTime(firstTime, { timeStyle: "short" })}</span><span>{formatDateTime(last!.time, { timeStyle: "short" })}</span></div>
    </> : <div className="flex h-40 items-center justify-center text-sm text-[var(--color-on-surface-variant)]">{t("noResourceSamples")}</div>}
  </SurfaceCard>;
}

export function ResourceTrendCards() {
  const { t } = useTranslation("observability");
  return <div className="grid gap-6 lg:grid-cols-2">
    <Trend name="process.cpu.utilization" title={t("cpuTrend")} factor={100} unit="%" color="var(--chart-1)" />
    <Trend name="process.memory.usage" title={t("memoryTrend")} factor={1 / 1048576} unit="MiB" color="var(--chart-2)" />
  </div>;
}

export function PluginCountCard() {
  const { t } = useTranslation("observability");
  const { formatNumber } = useLocaleFormatters();
  const client = useQueryClient();
  const plugins = useQuery({ queryKey: ["capability-plugin-counts"], queryFn: ({ signal }) => getCapabilityPluginCounts(signal), retry: false, refetchInterval: 10000 });
  const inventory = useQuery({
    queryKey: ["observability-inventory-counts"],
    queryFn: async ({ signal }) => {
      const [tools, skills, internalAgents, systemAgents, dudes] = await Promise.all([
        listTools({}, signal),
        fetchRegistrySkills({}, signal),
        fetchWorkspaceInternalAgents(PLATFORM_INTERNAL_AGENT_SCOPE, signal),
        fetchPlatformSystemAgents(signal),
        listDudes(signal),
      ]);
      return {
        tools: tools.length,
        skills: skills.length,
        agents: internalAgents.length + systemAgents.length,
        dudes: dudes.length,
      };
    },
    retry: false,
    refetchInterval: 60000,
  });
  useQueryErrorToast(plugins.error);
  useQueryErrorToast(inventory.error);
  useEffect(() => {
    const refresh = () => {
      void client.invalidateQueries({ queryKey: ["capability-plugin-counts"] });
      void client.invalidateQueries({ queryKey: ["observability-inventory-counts"] });
    };
    window.addEventListener(CAPABILITY_RESOURCES_CHANGED, refresh);
    return () => window.removeEventListener(CAPABILITY_RESOURCES_CHANGED, refresh);
  }, [client]);
  const counts = [
    ["pluginCount", plugins.data?.total],
    ["enabledPluginCount", plugins.data?.enabled],
    ["toolCount", inventory.data?.tools],
    ["skillCount", inventory.data?.skills],
    ["agentCount", inventory.data?.agents],
    ["dudeCount", inventory.data?.dudes],
  ] as const;
  return <SurfaceCard className="!shadow-none">
    <dl className="grid grid-cols-2 gap-x-4 gap-y-6 sm:grid-cols-3">
      {counts.map(([key, count]) => <div key={key}>
        <dt className="text-sm text-[var(--color-on-surface-variant)]">{t(key)}</dt>
        <dd className="mt-2 text-3xl font-semibold tabular-nums">{count == null ? "—" : formatNumber(count)}</dd>
      </div>)}
    </dl>
  </SurfaceCard>;
}
