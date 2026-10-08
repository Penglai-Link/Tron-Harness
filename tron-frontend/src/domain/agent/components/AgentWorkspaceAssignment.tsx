import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { requestApi } from "@/shared/api/httpClient";
import { Button } from "@/components/ui/button";
import { WorkspaceAssignmentDialog } from "@/domain/settings/components/WorkspaceAssignmentDialog";
import { notifyInternalAgentRegistryChanged } from "../model";

export function AgentWorkspaceAssignment({ code, name }: { code: string; name: string }) {
  const { t } = useTranslation("pages");
  const [open, setOpen] = useState(false);
  const cache = useQueryClient();
  const query = useQuery({ queryKey: ["agent-workspace-grants", code], enabled: open,
    queryFn: ({ signal }) => requestApi<{ workspaceCodes: string[] }>("GET /api/v1/agents/registry/{code}/workspace-grants", { pathParams: { code }, signal }),
  });
  if (code === "main-agent") return <span className="text-xs text-[var(--color-on-surface-variant)]">{t("platformData.builtinAgent")}</span>;
  return <>
    <Button variant="outline" size="sm" onClick={() => setOpen(true)}>{t("platformData.assign")}</Button>
    {open && query.data ? <WorkspaceAssignmentDialog name={name} assigned={query.data.workspaceCodes}
      save={(workspaceCodes) => requestApi("PUT /api/v1/agents/registry/{code}/workspace-grants", { pathParams: { code }, body: { workspaceCodes } })}
      onClose={() => setOpen(false)} onSaved={() => { void cache.invalidateQueries({ queryKey: ["agent-workspace-grants", code] }); notifyInternalAgentRegistryChanged(); }} /> : null}
    {open && query.error ? <p role="alert">{query.error.message}</p> : null}
  </>;
}
