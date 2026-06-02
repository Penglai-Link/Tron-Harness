import { useTranslation } from "react-i18next";

import { getAgentStatusTone } from "@/domain/agent/constants/agentManagementOptions";
import type { AgentStatus } from "@/domain/agent/model";
import { Badge } from "@/shared/ui/Badge";

export function AgentStatusBadge({ status }: { status: AgentStatus }) {
  const { t } = useTranslation("common");
  const label = (() => {
    switch (status) {
      case "Enabled":
        return t("agentStatus.enabled");
      case "Disabled":
        return t("agentStatus.disabled");
      case "Unavailable":
        return t("agentStatus.unavailable");
      default:
        return status;
    }
  })();

  return (
    <Badge tone={getAgentStatusTone(status)}>
      {label}
    </Badge>
  );
}
