import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Field } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { SearchField } from "@/components/search-field";
import { InfoTooltip } from "@/components/info-tooltip";
import { useToast } from "@/components/ui/toast";
import { listAgentKnowledgeLibraries, setAgentKnowledgeLibraries, type AgentKnowledgeLibrary } from "@/domain/knowledge-base/api/knowledgeBaseApi";
import { useKnowledgeUser, useKnowledgeViewActive } from "@/domain/knowledge-base/hooks/useKnowledgeLibraries";

interface Props { workspaceCode: string; agentCode: string; agentName: string; disabled: boolean }

export function InternalAgentKnowledgeField({ workspaceCode, agentCode, agentName, disabled }: Props) {
  const { t } = useTranslation("pages");
  const user = useKnowledgeUser();
  const [open, setOpen] = useState(false);
  const text = (key: string, options?: Record<string, unknown>) => t(`workspaceAgents.internal.knowledge.${key}`, options);
  const query = useQuery({
    queryKey: ["knowledge-agent-libraries", user, workspaceCode, agentCode],
    enabled: Boolean(user && workspaceCode),
    queryFn: ({ signal }) => listAgentKnowledgeLibraries(workspaceCode, agentCode, signal),
  });
  const selected = query.data?.filter((library) => library.granted) ?? [];

  return <><Field className="w-full lg:col-span-2" label={<span className="inline-flex items-center gap-1.5">{text("label")}<InfoTooltip ariaLabel={text("helpLabel")}><p>{text("help")}</p></InfoTooltip></span>}
    actions={<Button size="sm" variant="outline" disabled={disabled || query.isPending || Boolean(query.error)} onClick={() => setOpen(true)}>{text("choose", { count: selected.length })}</Button>}>
    {query.isPending ? <p role="status" className="text-sm text-[var(--color-on-surface-variant)]">{text("loading")}</p> : query.error ? <div className="flex items-center gap-2"><p role="alert" className="text-sm text-[var(--color-error)]">{query.error.message}</p><Button size="sm" variant="outline" onClick={() => void query.refetch()}>{text("retry")}</Button></div> : <p className="text-sm text-[var(--color-on-surface-variant)]">{selected.length ? selected.map((library) => library.name).join(" · ") : text("none")}</p>}
  </Field>
    {open && query.data ? <KnowledgeSelectionDialog key={user} workspaceCode={workspaceCode} agentCode={agentCode} agentName={agentName} libraries={query.data} onClose={() => setOpen(false)} /> : null}
  </>;
}

function KnowledgeSelectionDialog({ workspaceCode, agentCode, agentName, libraries, onClose }: Omit<Props, "disabled"> & { libraries: AgentKnowledgeLibrary[]; onClose: () => void }) {
  const { t } = useTranslation("pages");
  const text = (key: string, options?: Record<string, unknown>) => t(`workspaceAgents.internal.knowledge.${key}`, options);
  const toast = useToast();
  const queryClient = useQueryClient();
  const active = useKnowledgeViewActive();
  const [selected, setSelected] = useState(() => libraries.filter((library) => library.granted).map((library) => library.id));
  const [search, setSearch] = useState("");
  const save = useMutation({
    mutationFn: () => setAgentKnowledgeLibraries(workspaceCode, agentCode, selected),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["knowledge-agent-libraries"] }),
        queryClient.invalidateQueries({ queryKey: ["knowledge-base"] }),
        queryClient.invalidateQueries({ queryKey: ["knowledge-bases"] }),
      ]);
      if (!active.current) return;
      toast.success(text("saved"));
      onClose();
    },
    onError: (error) => { if (active.current) toast.error(error.message); },
  });
  const keyword = search.trim().toLocaleLowerCase();
  const visible = libraries.filter((library) => `${library.name} ${library.description}`.toLocaleLowerCase().includes(keyword));

  return <Dialog open onOpenChange={(value) => { if (!value && !save.isPending) onClose(); }}>
    <DialogContent className="flex max-h-[85dvh] flex-col overflow-hidden sm:max-w-2xl" closeLabel={text("close")}>
      <DialogHeader><DialogTitle>{text("title")}</DialogTitle><DialogDescription>{agentName} · {text("selected", { count: selected.length })}</DialogDescription></DialogHeader>
      <SearchField className="w-full" ariaLabel={text("search")} placeholder={text("search")} value={search} onChange={setSearch} />
      <div className="tron-scrollbar min-h-0 space-y-2 overflow-y-auto">
        {visible.map((library) => <label key={library.id} className="flex cursor-pointer items-start gap-3 rounded-xl border border-[var(--color-outline-variant)] p-3">
          <Switch aria-labelledby="" aria-label={library.name} checked={selected.includes(library.id)} disabled={save.isPending} onCheckedChange={(checked) => setSelected((current) => checked ? [...new Set([...current, library.id])] : current.filter((id) => id !== library.id))} />
          <span className="min-w-0"><span className="block text-sm font-medium">{library.name}</span>{library.description ? <span className="mt-1 block text-sm text-[var(--color-on-surface-variant)]">{library.description}</span> : null}<span className="mt-1 block text-xs text-[var(--color-on-surface-variant)]">{text("documents", { count: library.documentCount })}</span></span>
        </label>)}
        {!visible.length ? <p className="py-5 text-sm text-[var(--color-on-surface-variant)]">{text(libraries.length ? "noMatches" : "empty")}</p> : null}
      </div>
      <DialogFooter><Button variant="outline" disabled={save.isPending} onClick={onClose}>{text("cancel")}</Button><Button disabled={save.isPending} onClick={() => save.mutate()}>{text(save.isPending ? "saving" : "confirm")}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}
