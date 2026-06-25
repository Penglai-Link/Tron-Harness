import type { ReactNode } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

import {
  type BadgeTone,
  type CorpusRecord,
  type ProfileRecord,
  type SourceRecord,
  type TextUnitRecord,
  type WarningRecord,
} from "@/domain/rag/model/RagModel";

export function statusTone(status: CorpusRecord["status"] | ProfileRecord["status"] | SourceRecord["status"] | TextUnitRecord["embeddingStatus"] | WarningRecord["severity"]): BadgeTone {
  switch (status) {
    case "Active":
    case "Ready":
    case "Indexed":
    case "Low":
      return "success";
    case "Indexing":
    case "Pending":
      return "accent";
    case "Warning":
    case "Needs Review":
    case "Medium":
    case "Skipped":
      return "warning";
    case "High":
      return "error";
    default:
      return "neutral";
  }
}

export function formatRagStatusLabel(status: string, t?: TFunction) {
  const statusLabels: Record<string, string> = {
    Active: "Active",
    COMPLETED: "Completed",
    Draft: "Draft",
    FAILED: "Failed",
    High: "High",
    Indexed: "Indexed",
    Indexing: "Indexing",
    Low: "Low",
    Medium: "Medium",
    "Needs Review": "Needs Review",
    Pending: "Pending",
    Ready: "Ready",
    Reserved: "Reserved",
    RUNNING: "Running",
    Skipped: "Skipped",
    SUCCESS: "Success",
    Warning: "Warning",
    completed: "Completed",
    failed: "Failed",
    running: "Running",
    success: "Success",
  };
  const key = status
    .replaceAll(" ", "")
    .replaceAll("-", "")
    .replaceAll("_", "")
    .toLowerCase();
  return t?.(`pages:rag.status.${key}`, { defaultValue: statusLabels[status] ?? status }) ?? statusLabels[status] ?? status;
}

export function formatCompactNumber(value: number, locale = "en-US") {
  return new Intl.NumberFormat(locale, { maximumFractionDigits: 1, notation: "compact" }).format(value);
}

export function SectionTitle({ description, icon, title }: { description?: string; icon: ReactNode; title: string }) {
  return (
    <div className="flex items-start gap-3">
      <span className="flex size-9 shrink-0 items-center justify-center rounded-xl bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
        {icon}
      </span>
      <div>
        <h2 className="text-base font-semibold">{title}</h2>
        {description ? <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{description}</p> : null}
      </div>
    </div>
  );
}

export function InspectorCard({ children }: { children: ReactNode }) {
  return (
    <div className="grid gap-3 rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-surface-container-low)] p-4 text-sm">
      {children}
    </div>
  );
}

export function InspectorSection({ children, title }: { children: ReactNode; title: string }) {
  return (
    <div>
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{title}</p>
      <div className="mt-3">{children}</div>
    </div>
  );
}

export function InspectorRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-[var(--color-on-surface-variant)]">{label}</span>
      <span className="text-right font-medium text-[var(--color-on-surface)]">{value}</span>
    </div>
  );
}

export function Pill({ children }: { children: ReactNode }) {
  return (
    <span className="inline-flex h-7 items-center rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] px-2.5 font-mono text-[12px] text-[var(--color-on-surface-variant)]">
      {children}
    </span>
  );
}

export function DiagnosticPreview({ rows }: { rows: [string, string][] }) {
  const { t } = useTranslation("pages");

  return (
    <div className="mt-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
        {t("rag.common.preview", { defaultValue: "Preview" })}
      </p>
      <div className="mt-3 grid gap-2 text-sm">
        {rows.map(([label, value]) => <InspectorRow key={label} label={label} value={value} />)}
      </div>
    </div>
  );
}

export function PreviewLine({ description, title, value }: { description?: string; title: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-3 py-2">
      <div>
        <p className="font-medium">{title}</p>
        {description ? <p className="text-[13px] text-[var(--color-on-surface-variant)]">{description}</p> : null}
      </div>
      <span className="font-mono text-sm font-semibold">{value}</span>
    </div>
  );
}
