import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Field } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { useAuthStore } from "@/store/authStore";
import { WorkspaceAssignmentDialog } from "@/domain/settings/components/WorkspaceAssignmentDialog";
import { usePlatformWorkspaces } from "@/domain/settings/components/PlatformWorkspaceScope";
import { loadAgentWorkspaceGrants } from "../hooks/agentWorkspaceGrants";

interface Props {
  code?: string;
  name: string;
  disabled: boolean;
  selectedCodes?: string[];
  onSelectionChange: (codes: string[]) => void;
}

/** Platform-only field. Selection belongs to the enclosing editor's draft. */
export function AgentWorkspaceField({ code, name, disabled, selectedCodes, onSelectionChange }: Props) {
  const { t } = useTranslation("pages");
  const user = useAuthStore((state) => state.session?.user.userCode ?? "");
  const [open, setOpen] = useState(false);
  const workspaces = usePlatformWorkspaces();
  const builtin = code === "main-agent";
  const grants = useQuery({
    queryKey: ["agent-workspace-grants", code, user],
    enabled: Boolean(code) && !builtin,
    queryFn: ({ signal }) => loadAgentWorkspaceGrants(code!, signal),
  });
  const selected = selectedCodes ?? (code ? grants.data?.workspaceCodes : workspaces.data?.filter((workspace) => workspace.status === "Active").map((workspace) => workspace.workspaceCode)) ?? [];
  const pending = Boolean(code) && !builtin && grants.isPending;
  const error = grants.error ?? workspaces.error;
  return <>
    <Field label={t("platformData.agentAuthorization")} actions={!builtin ? (
      <Button type="button" size="sm" variant="outline" disabled={disabled || pending || workspaces.isPending || Boolean(error)} onClick={() => setOpen(true)}>
        {t("platformData.chooseWorkspaces", { count: selected.length })}
      </Button>
    ) : undefined}>
      {builtin ? <p className="text-sm text-[var(--color-on-surface-variant)]">{t("platformData.builtinAgent")}</p>
        : error ? <div className="flex items-center gap-2"><p role="alert" className="text-sm text-[var(--color-error)]">{error.message}</p><Button type="button" size="sm" variant="outline" disabled={disabled} onClick={() => { if (code) void grants.refetch(); void workspaces.refetch(); }}>{t("platformData.retry")}</Button></div>
          : pending || workspaces.isPending ? <p role="status">{t("platformData.loadingWorkspaces")}</p>
            : <p className="text-sm text-[var(--color-on-surface-variant)]">{selected.length ? selected.map((value) => { const option = workspaces.data?.find((workspace) => workspace.workspaceCode === value); return option?.label || option?.name || value; }).join(" · ") : t("platformData.unassigned")}</p>}
    </Field>
    {open ? <WorkspaceAssignmentDialog name={name} assigned={selected}
      confirmLabel={t("platformData.confirm")}
      help={t("platformData.draftAssignmentHelp")}
      save={async (codes) => { onSelectionChange(codes); }}
      onClose={() => setOpen(false)} onSaved={() => {}} /> : null}
  </>;
}
