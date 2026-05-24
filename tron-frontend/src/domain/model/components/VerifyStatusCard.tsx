import { Badge } from "@/shared/ui/Badge";
import { formatModelVerifyStatus, getModelVerifyTone } from "@/domain/model/constants/modelPageOptions";
import type { ModelRecord } from "@/domain/model/model";

function formatTimestamp(value: string | null) {
  if (!value) return "Never";
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

export function VerifyStatusCard({ record }: { record: ModelRecord | null }) {
  const statusLabel = record ? formatModelVerifyStatus(record.lastVerifyStatus) : "No model selected";

  return (
    <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="text-sm font-medium text-[var(--color-on-surface)]">{statusLabel}</p>
          <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">Last checked: {formatTimestamp(record?.lastVerifiedAt ?? null)}</p>
        </div>
        {record ? (
          <Badge tone={getModelVerifyTone(record.lastVerifyStatus)}>{statusLabel}</Badge>
        ) : null}
      </div>
      {record?.lastVerifyMessage ? <p className="mt-3 text-sm text-[var(--color-on-surface-variant)]">{record.lastVerifyMessage}</p> : null}
    </div>
  );
}
