import { useTranslation } from "react-i18next";
import { Cpu, Server } from "lucide-react";
import { useLocaleFormatters } from "@/shared";
import type { ServiceProcess } from "../model/Trace";

export function ServiceTopology({
  processes,
  sampledAt,
}: {
  processes: Array<Omit<ServiceProcess, "memoryBytes"> & { memoryBytes: number | null }>;
  sampledAt: number;
}) {
  const { t } = useTranslation("observability");
  const { formatNumber } = useLocaleFormatters();
  const render = (process: (typeof processes)[number], ancestors: Set<number>) => {
    if (ancestors.has(process.pid)) return null;
    const visited = new Set(ancestors).add(process.pid);
    const children = processes.filter((p) => p.parentPid === process.pid);
    const Icon = process.parentPid == null ? Server : Cpu;
    return (
      <li
        key={process.pid}
        className={
          process.parentPid == null
            ? "min-w-0"
            : "relative min-w-0 before:absolute before:top-7 before:-left-5 before:w-5 before:border-t before:border-[var(--color-outline-variant)]"
        }
      >
        <div className="flex items-center gap-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-3">
          <Icon className="size-5 shrink-0 text-[var(--color-primary)]" />
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-medium" title={process.name}>
              {process.name}
            </p>
            <p className="text-xs text-[var(--color-on-surface-variant)]">
              PID {process.pid}
              {process.parentPid != null && <> · {t("childProcess")}</>}
            </p>
          </div>
          <span
            className="shrink-0 text-xs tabular-nums"
            title={t("processUptime", {
              seconds: Math.max(
                0,
                Math.floor((sampledAt - process.startedAt) / 1000)
              ),
            })}
          >
            {process.memoryBytes == null ? "—" : formatNumber(process.memoryBytes / 1048576, {
              maximumFractionDigits: 1,
            })}{process.memoryBytes == null ? "" : " MiB"}
          </span>
        </div>
        {children.length > 0 && (
          <ul className="ml-5 mt-3 space-y-3 border-l border-[var(--color-outline-variant)] pl-5">
            {children.map((child) => render(child, visited))}
          </ul>
        )}
      </li>
    );
  };
  const ids = new Set(processes.map((p) => p.pid));
  const roots = processes.filter(
    (p) => p.parentPid == null || !ids.has(p.parentPid)
  );
  return (
    <ul aria-label={t("serviceTopology")} className="space-y-3">
      {roots.map((p) => render(p, new Set()))}
    </ul>
  );
}
