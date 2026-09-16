import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import type {
  ModelProviderConnection,
  ProviderSyncRun,
} from "@/domain/model/api";

function providerStatusVariant(status: ModelProviderConnection["status"]) {
  switch (status) {
    case "Connected":
      return "success" as const;
    case "Degraded":
      return "warning" as const;
    case "AuthRequired":
      return "error" as const;
    case "PendingAuthorization":
      return "info" as const;
    default:
      return "outline" as const;
  }
}

function syncStatusVariant(status: ProviderSyncRun["status"]) {
  switch (status) {
    case "Succeeded":
      return "success" as const;
    case "Failed":
      return "error" as const;
    default:
      return "info" as const;
  }
}

export function ModelProviderStatusBadge({
  status,
}: {
  status: ModelProviderConnection["status"];
}) {
  const { t } = useTranslation("pages");
  return (
    <Badge variant={providerStatusVariant(status)}>
      {t(`modelProviders.status.${status}`)}
    </Badge>
  );
}

export function ProviderSyncStatusBadge({
  status,
}: {
  status: ProviderSyncRun["status"];
}) {
  const { t } = useTranslation("pages");
  return (
    <Badge variant={syncStatusVariant(status)}>
      {t(`modelProviders.status.${status}`)}
    </Badge>
  );
}
