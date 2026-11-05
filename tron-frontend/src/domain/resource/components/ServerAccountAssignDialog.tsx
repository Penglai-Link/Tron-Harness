import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { SearchField } from "@/components/search-field";
import { toastManager } from "@/components/ui/toast";
import { assignServerAccount } from "@/domain/resource/api";
import { useManagedResourceRecords } from "@/domain/resource/hooks";
import { updateMockResource } from "@/domain/resource/mock";
import type { ResourceRecord } from "@/domain/resource/model";
import { serverAccountBindings } from "@/domain/resource/model/serverAccount";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

interface Props {
  record: ResourceRecord;
  workspaceCode: string;
  workspaceName: string;
  onClose: () => void;
  onSaved: () => void;
}

// Mounted only while open so every visit starts from the latest saved assignments.
export function ServerAccountAssignDialog({ record, workspaceCode, workspaceName, onClose, onSaved }: Props) {
  const { t } = useTranslation("pages");
  const pageT = (key: string, options?: Record<string, unknown>) => t(`resourceConnections.${key}`, options);
  const [selected, setSelected] = useState(() => new Set(serverAccountBindings(record).map((server) => `${server.workspaceCode}:${server.serverCode}`)));
  const [search, setSearch] = useState("");
  const [saving, setSaving] = useState(false);
  const servers = useManagedResourceRecords({ kinds: ["SERVER_CONNECTION"], search: "", workspaceCode, workspaceName });
  const query = search.trim().toLowerCase();
  const options = servers.data.filter((server) => [server.name, server.resourceCode, server.attributes.host]
    .some((value) => typeof value === "string" && value.toLowerCase().includes(query)));
  const unavailable = [...selected].filter((code) => !servers.data.some((server) => `${server.workspaceCode}:${server.resourceCode}` === code));

  async function save() {
    if (saving || servers.loading || servers.error) return;
    setSaving(true);
    try {
      const selectedServers = servers.data.filter((server) => selected.has(`${server.workspaceCode}:${server.resourceCode}`));
      const serverCodes = selectedServers.map((server) => server.resourceCode);
      if (getServiceDataSource("resource") === "api") {
        await assignServerAccount(record.resourceCode, { workspaceCode, serverCodes });
      } else if (!updateMockResource(workspaceCode, record.resourceCode, {
        allowInheritance: record.allowInheritance,
        attributes: { ...record.attributes, servers: selectedServers.map((server) => ({ workspaceCode: server.workspaceCode, serverCode: server.resourceCode })) },
        description: record.description,
        name: record.name,
      })) {
        throw new Error(pageT("serverAccount.saveFailed"));
      }
      toastManager.add({ title: pageT("serverAccount.assigned"), type: "success" });
      onSaved();
      onClose();
    } catch (cause) {
      toastManager.add({ title: pageT("serverAccount.saveFailed"), description: cause instanceof Error ? cause.message : undefined, type: "error" });
    } finally {
      setSaving(false);
    }
  }

  function toggle(code: string, checked: boolean) {
    setSelected((current) => {
      const next = new Set(current);
      if (checked) next.add(code); else next.delete(code);
      return next;
    });
  }

  return (
    <Dialog open onOpenChange={(open) => { if (!open && !saving) onClose(); }}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{pageT("serverAccount.assignTitle", { name: record.name })}</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">{pageT("serverAccount.assignHint")}</p>
        <SearchField ariaLabel={pageT("server.searchPlaceholder")} placeholder={pageT("server.searchPlaceholder")} value={search} onChange={setSearch} />
        <fieldset disabled={saving || servers.loading} className="max-h-80 overflow-y-auto divide-y divide-border rounded-lg border">
          {servers.loading ? <p className="p-4 text-sm">{pageT("serverAccount.loading")}</p> : servers.error ? (
            <div className="p-4"><p className="text-sm text-destructive">{servers.error.message}</p><Button variant="outline" onClick={servers.refresh}>{pageT("actions.refresh")}</Button></div>
          ) : <>
            {options.map((server) => (
              <label key={server.resourceCode} className="flex cursor-pointer items-center gap-3 p-3 hover:bg-muted/40">
                <input type="checkbox" className="size-4 accent-[var(--color-primary)]" checked={selected.has(`${server.workspaceCode}:${server.resourceCode}`)} onChange={(event) => toggle(`${server.workspaceCode}:${server.resourceCode}`, event.target.checked)} />
                <span className="min-w-0"><span className="block truncate text-sm font-medium">{server.name}</span><span className="block truncate text-xs text-muted-foreground">{String(server.attributes.host ?? server.resourceCode)}{typeof server.attributes.port === "number" ? `:${server.attributes.port}` : ""}</span></span>
              </label>
            ))}
            {unavailable.map((code) => (
              <label key={code} className="flex items-center gap-3 p-3 text-sm text-muted-foreground">
                <input type="checkbox" className="size-4" checked onChange={() => toggle(code, false)} />
                {code.split(":").slice(1).join(":")} · {pageT("serverAccount.unavailable")}
              </label>
            ))}
            {!options.length && !unavailable.length ? <p className="p-4 text-sm text-muted-foreground">{pageT(query ? "serverAccount.noMatches" : "server.empty")}</p> : null}
          </>}
        </fieldset>
        <DialogFooter>
          <span className="mr-auto text-sm text-muted-foreground">{pageT("serverAccount.selectedCount", { count: selected.size })}</span>
          <Button disabled={saving} variant="outline" onClick={onClose}>{pageT("actions.cancel")}</Button>
          <Button disabled={saving || servers.loading || !!servers.error || unavailable.length > 0} onClick={() => void save()}>{pageT(saving ? "serverAccount.saving" : "serverAccount.save")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
