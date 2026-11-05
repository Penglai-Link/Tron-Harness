import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";

import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { toastManager } from "@/components/ui/toast";
import { verifyServerAccount } from "@/domain/resource/api";
import { ServerAccountCreateDialog } from "@/domain/resource/components/ServerAccountCreateDialog";
import { ServerAccountAssignDialog } from "@/domain/resource/components/ServerAccountAssignDialog";
import { useManagedResourceRecords, useResourceDetail } from "@/domain/resource/hooks";
import { serverAccountBindings } from "@/domain/resource/model/serverAccount";
import { PageHeader, SurfaceCard, useWorkspaceSpaces } from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function ServerAccountDetailPage() {
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const [params] = useSearchParams();
  const resourceCode = params.get("resourceCode") ?? "";
  return <AccountDetail key={`${workspace?.workspaceCode}:${resourceCode}`} workspaceCode={workspace?.workspaceCode ?? ""} workspaceName={workspace?.name ?? ""} resourceCode={resourceCode} />;
}

function AccountDetail({ workspaceCode, workspaceName, resourceCode }: { workspaceCode: string; workspaceName: string; resourceCode: string }) {
  const { t } = useTranslation("pages");
  const pageT = (key: string) => t(`resourceConnections.${key}`);
  const detail = useResourceDetail({ resourceCode, workspaceCode, workspaceName });
  const servers = useManagedResourceRecords({ kinds: ["SERVER_CONNECTION"], search: "", workspaceCode, workspaceName });
  const [editing, setEditing] = useState(false);
  const [assigning, setAssigning] = useState(false);
  const [verifying, setVerifying] = useState<string | null>(null);
  const record = detail.data?.kind === "SERVER_ACCOUNT" ? detail.data : null;
  const writable = record && record.sourceKind !== "INHERITED";
  const bindings = record ? serverAccountBindings(record) : [];

  async function verify(serverCode: string) {
    if (verifying) return;
    setVerifying(serverCode);
    try {
      const result = await verifyServerAccount(resourceCode, { workspaceCode, serverCode });
      if (!result.verified) throw new Error(pageT("messages.connectionFailed"));
      toastManager.add({ title: pageT("messages.connectionVerified"), type: "success" });
    } catch (cause) {
      toastManager.add({ title: pageT("messages.connectionFailed"), description: cause instanceof Error ? cause.message : undefined, type: "error" });
    } finally {
      setVerifying(null);
    }
  }

  return <>
    <PageHeader title={record?.name ?? pageT("serverAccount.detailTitle")} breadcrumbs={[{ href: "/resource-management/server-accounts", label: pageT("serverAccount.title") }, { label: resourceCode }]} actions={writable ? <>
      <Button variant="outline" onClick={() => setEditing(true)}>{pageT("actions.edit")}</Button>
      <Button onClick={() => setAssigning(true)}>{pageT("serverAccount.assign")}</Button>
    </> : undefined} />
    {record ? <SurfaceCard className="max-w-5xl space-y-6">
      <dl className="grid gap-x-8 gap-y-5 sm:grid-cols-2">
        {[
          [pageT("fields.accountCode"), record.resourceCode],
          [pageT("fields.username"), String(record.attributes?.username ?? "—")],
          [pageT("fields.authenticationType"), pageT(record.attributes?.authenticationType === "PrivateKey" ? "authentication.privateKey" : "authentication.password")],
          [pageT("serverAccount.credential"), pageT(record.attributes?.credentialConfigured === true ? "serverAccount.configured" : "serverAccount.notConfigured")],
        ].map(([label, value]) => <div key={label} className="min-w-0"><dt className="text-xs text-muted-foreground">{label}</dt><dd className="mt-1 break-all text-sm">{value}</dd></div>)}
      </dl>
      {record.description ? <p className="whitespace-pre-wrap text-sm text-muted-foreground">{record.description}</p> : null}
      <section className="border-t pt-5">
        <h2 className="mb-3 text-sm font-semibold">{pageT("serverAccount.servers")} <Badge variant="outline">{bindings.length}</Badge></h2>
        {!bindings.length ? <p className="text-sm text-muted-foreground">{pageT("serverAccount.unassignedHint")}</p> : (
          <ul className="divide-y divide-border">
            {bindings.map((binding) => {
              const server = servers.data.find((candidate) => candidate.resourceCode === binding.serverCode && candidate.workspaceCode === binding.workspaceCode);
              return <li key={`${binding.workspaceCode}:${binding.serverCode}`} className="flex items-center justify-between gap-4 py-3">
                <div className="min-w-0"><p className="truncate text-sm font-medium">{server?.name ?? binding.serverCode}</p><p className="truncate text-xs text-muted-foreground">{server ? `${String(server.attributes.host ?? "")}:${String(server.attributes.port ?? 22)}` : pageT("serverAccount.unavailable")}</p></div>
                <Button variant="outline" size="sm" disabled={!!verifying || !server || getServiceDataSource("resource") !== "api"} onClick={() => void verify(binding.serverCode)}>{pageT(verifying === binding.serverCode ? "actions.verifying" : "serverAccount.verify")}</Button>
              </li>;
            })}
          </ul>
        )}
      </section>
    </SurfaceCard> : <p className="py-8 text-sm text-muted-foreground">{!workspaceCode ? pageT("workspaceRequired") : detail.loading ? pageT("serverAccount.loading") : detail.error?.message ?? pageT("serverAccount.notFound")}</p>}
    {record && editing ? <ServerAccountCreateDialog open record={record} onOpenChange={setEditing} onCreated={detail.refresh} workspaceCode={workspaceCode} /> : null}
    {record && assigning ? <ServerAccountAssignDialog record={record} workspaceCode={workspaceCode} workspaceName={workspaceName} onClose={() => setAssigning(false)} onSaved={detail.refresh} /> : null}
  </>;
}
