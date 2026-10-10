import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { loadAgentWorkspaceGrants, persistAgentWorkspaceGrants } from "../hooks/agentWorkspaceGrants";
import { useAuthStore } from "@/store/authStore";
import { Button } from "@/components/ui/button";
import { WorkspaceAssignmentDialog } from "@/domain/settings/components/WorkspaceAssignmentDialog";
import { notifyInternalAgentRegistryChanged } from "../model";

export function AgentWorkspaceAssignment({ code, name }: { code: string; name: string }) {
  const { t } = useTranslation("pages");
  const [open, setOpen] = useState(false);
  const cache = useQueryClient();
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const query = useQuery({ queryKey: ["agent-workspace-grants", code, user], enabled: open && code !== "main-agent",
    queryFn: ({ signal }) => loadAgentWorkspaceGrants(code, signal),
  });
  if (code === "main-agent") return <span className="text-xs text-[var(--color-on-surface-variant)]">{t("platformData.builtinAgent")}</span>;
  return <>
    <Button variant="outline" size="sm" onClick={() => setOpen(true)}>{t("platformData.assign")}</Button>
    {open && query.data ? <WorkspaceAssignmentDialog name={name} assigned={query.data.workspaceCodes}
      save={(workspaceCodes) => persistAgentWorkspaceGrants(code, workspaceCodes)}
      onClose={() => setOpen(false)} onSaved={() => { void cache.invalidateQueries({ queryKey: ["agent-workspace-grants", code] }); notifyInternalAgentRegistryChanged(); }} /> : null}
    {open && query.error ? <p role="alert">{query.error.message}</p> : null}
  </>;
}
