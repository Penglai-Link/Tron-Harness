import { useTranslation } from "react-i18next";

import type { ResourceVersionRecord } from "@/domain/resource/model";
import { Badge } from "@/shared/ui/Badge";

interface ResourceVersionSourceProps {
  inline?: boolean;
  record: ResourceVersionRecord;
}

export function ResourceVersionSource({ inline = false, record }: ResourceVersionSourceProps) {
  const { t } = useTranslation("pages");
  const showEffectiveSource = record.lifecycleStatus === "ACTIVE";
  const value = showEffectiveSource ? record.sourceKind : record.scopeKind;
  const tone = value === "LOCAL"
    ? "success"
    : value === "INHERITED"
      ? "info"
      : value === "OVERRIDE"
        ? "warning"
        : "outline";
  const owner = record.scopeKind === "PLATFORM"
    ? t("resources.scope.platform")
    : record.ownerWorkspaceName ?? record.ownerWorkspaceCode ?? t("resources.scope.workspace");

  if (inline) {
    return (
      <div className="flex min-w-0 items-center gap-1.5">
        <Badge tone={tone}>{t(`resources.source.${value.toLowerCase()}`)}</Badge>
        <span className="truncate text-[11px] text-[var(--color-on-surface-variant)]" title={owner}>
          {owner}
        </span>
      </div>
    );
  }

  return (
    <div className="min-w-0">
      <Badge tone={tone}>{t(`resources.source.${value.toLowerCase()}`)}</Badge>
      <p className="mt-1 truncate text-[12px] text-[var(--color-on-surface-variant)]" title={owner}>
        {owner}
      </p>
    </div>
  );
}
