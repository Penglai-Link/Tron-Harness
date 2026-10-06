import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import { setKnowledgeLibraryGrants } from "../api/knowledgeBaseApi";
import { useKnowledgeAgents, useKnowledgeViewActive } from "../hooks/useKnowledgeLibraries";

export function KnowledgeGrantsDialog({ workspace, libraryId, grantedCodes, onClose, onChanged }: {
  workspace: string; libraryId: string; grantedCodes: string[]; onClose: () => void; onChanged: () => Promise<unknown>;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const toast = useToast();
  const active = useKnowledgeViewActive();
  const agents = useKnowledgeAgents(workspace, true);
  const [selected, setSelected] = useState([...grantedCodes]);
  const save = useMutation({ mutationFn: () => setKnowledgeLibraryGrants(workspace, libraryId, selected),
    onSuccess: async () => { if (!active.current) return; await onChanged(); if (!active.current) return; toast.success(text("saved")); onClose(); },
    onError: (error) => { if (active.current) toast.error(error.message); },
  });
  const missing = selected.filter((code) => !agents.data?.some((agent) => agent.agentCode === code));
  function toggle(code: string, checked: boolean) {
    setSelected((codes) => checked ? [...new Set([...codes, code])] : codes.filter((item) => item !== code));
  }
  return <Dialog open onOpenChange={(open) => { if (!open && !save.isPending) onClose(); }}>
    <DialogContent closeLabel={text("cancel")} className="sm:max-w-lg"><DialogHeader><DialogTitle>{text("grants")}</DialogTitle><DialogDescription>{text("grantsHelp")}</DialogDescription></DialogHeader>
      {agents.isLoading ? <p role="status">{text("loading")}</p> : null}
      {agents.error ? <p role="alert" className="text-[var(--color-error)]">{agents.error.message}</p> : null}
      <div className="max-h-80 overflow-y-auto space-y-3">
        {(agents.data ?? []).map((agent) => <label key={agent.agentCode} className="flex items-center justify-between gap-4"><span>{agent.name}<span className="block text-xs text-[var(--color-on-surface-variant)]">{agent.agentCode}</span></span><Switch checked={selected.includes(agent.agentCode)} disabled={save.isPending} onCheckedChange={(checked) => toggle(agent.agentCode, checked)} aria-label={agent.name} /></label>)}
        {!agents.isLoading && !agents.error && !agents.data?.length ? <p>{text("noAgents")}</p> : null}
        {!agents.isLoading && !agents.error ? missing.map((code) => <label key={code} className="flex items-center justify-between gap-4"><span>{code} <span className="text-xs">{text("unavailableAgent")}</span></span><Switch checked disabled={save.isPending} onCheckedChange={(checked) => toggle(code, checked)} aria-label={code} /></label>) : null}
      </div>
      <DialogFooter><Button variant="outline" disabled={save.isPending} onClick={onClose}>{text("cancel")}</Button><Button disabled={save.isPending || agents.isLoading || Boolean(agents.error)} onClick={() => save.mutate()}>{text(save.isPending ? "saving" : "save")}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}
