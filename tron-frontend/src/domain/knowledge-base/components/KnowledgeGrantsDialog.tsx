import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import { assignKnowledgeLibraryAgents, getKnowledgeLibraryAgentGrants } from "../api/knowledgeBaseApi";
import { useKnowledgeAgents, useKnowledgeUser, useKnowledgeViewActive } from "../hooks/useKnowledgeLibraries";

export function KnowledgeGrantsDialog({ libraryId, onClose, onChanged }: { libraryId: string; onClose: () => void; onChanged: () => Promise<unknown> }) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const toast = useToast();
  const queryClient = useQueryClient();
  const user = useKnowledgeUser();
  const active = useKnowledgeViewActive();
  const agents = useKnowledgeAgents(true);
  const grants = useQuery({ queryKey: ["knowledge-library-agent-grants", user, libraryId], queryFn: ({ signal }) => getKnowledgeLibraryAgentGrants(libraryId, signal) });
  const [draft, setDraft] = useState<string[] | null>(null);
  const selected = draft ?? grants.data?.agentCodes ?? [];
  const save = useMutation({ mutationFn: () => assignKnowledgeLibraryAgents(libraryId, selected),
    onSuccess: async () => {
      await Promise.all([queryClient.invalidateQueries({ queryKey: ["knowledge-agent-libraries"] }), queryClient.invalidateQueries({ queryKey: ["knowledge-bases"] }), queryClient.invalidateQueries({ queryKey: ["knowledge-base"] }), queryClient.invalidateQueries({ queryKey: ["platform-knowledge-bases"] }), queryClient.invalidateQueries({ queryKey: ["knowledge-library-agent-grants", user, libraryId] })]);
      if (!active.current) return;
      await onChanged(); if (!active.current) return;
      toast.success(text("saved")); onClose();
    },
    onError: (error) => { if (active.current) toast.error(error.message); },
  });
  const loading = agents.isLoading || grants.isLoading;
  const error = agents.error ?? grants.error;
  const missing = selected.filter((code) => !agents.data?.some((agent) => agent.agentCode === code));
  function toggle(code: string, checked: boolean) {
    setDraft(checked ? [...new Set([...selected, code])] : selected.filter((item) => item !== code));
  }
  return <Dialog open onOpenChange={(open) => { if (!open && !save.isPending) onClose(); }}>
    <DialogContent closeLabel={text("cancel")} className="sm:max-w-lg"><DialogHeader><DialogTitle>{text("grants")}</DialogTitle><DialogDescription>{text("grantsHelp")}</DialogDescription></DialogHeader>
    {loading ? <p role="status">{text("loading")}</p> : null}
    {error ? <p role="alert" className="text-[var(--color-error)]">{error.message}</p> : null}
    {!loading && !error ? <div className="max-h-80 overflow-y-auto space-y-3">
      {(agents.data ?? []).map((agent) => <label key={agent.agentCode} className="flex items-center justify-between gap-4"><span>{agent.name}<span className="block text-xs text-[var(--color-on-surface-variant)]">{agent.agentCode}</span></span><Switch aria-labelledby="" checked={selected.includes(agent.agentCode)} disabled={save.isPending} onCheckedChange={(checked) => toggle(agent.agentCode, checked)} aria-label={agent.name} /></label>)}
      {!agents.data?.length ? <p>{text("noAgents")}</p> : null}
      {missing.map((code) => <label key={code} className="flex items-center justify-between gap-4"><span>{code} <span className="text-xs">{text("unavailableAgent")}</span></span><Switch aria-labelledby="" checked disabled={save.isPending} onCheckedChange={(checked) => toggle(code, checked)} aria-label={code} /></label>)}
    </div> : null}
    <DialogFooter><Button variant="outline" disabled={save.isPending} onClick={onClose}>{text("cancel")}</Button><Button disabled={save.isPending || loading || !grants.data || Boolean(error)} onClick={() => save.mutate()}>{text(save.isPending ? "saving" : "save")}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}
