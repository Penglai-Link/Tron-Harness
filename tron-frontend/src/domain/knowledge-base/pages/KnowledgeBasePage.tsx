import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { MoreHorizontal } from "lucide-react";
import { PageHeader, SurfaceCard } from "@/shared";
import { Field, TextInput, TextArea } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useToast } from "@/components/ui/toast";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { createKnowledgeLibrary, deleteKnowledgeLibrary } from "../api/knowledgeBaseApi";
import type { KnowledgeLibrarySummary } from "../model/KnowledgeLibraryModel";
import { useKnowledgeLibraries, useKnowledgeWorkspace, useKnowledgeUser, useKnowledgeViewActive } from "../hooks/useKnowledgeLibraries";

export function KnowledgeBasePage() {
  const workspace = useKnowledgeWorkspace();
  const user = useKnowledgeUser();
  // Remount drafts when switching workspace; never carry an upload or grant draft across spaces.
  return <KnowledgeCatalog key={`${user}:${workspace}`} workspace={workspace} />;
}

function KnowledgeCatalog({ workspace }: { workspace: string }) {
  const { t, i18n } = useTranslation("pages");
  const active = useKnowledgeViewActive();
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const navigate = useNavigate();
  const toast = useToast();
  const user = useKnowledgeUser();
  const cache = useQueryClient();
  const query = useKnowledgeLibraries(workspace);
  const [filter, setFilter] = useState("");
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<KnowledgeLibrarySummary | null>(null);
  const deletion = useMutation({
    mutationFn: (library: KnowledgeLibrarySummary) => deleteKnowledgeLibrary(workspace, library.id),
    onSuccess: async (_, library) => {
      cache.removeQueries({ queryKey: ["knowledge-base", user, workspace, library.id] });
      await cache.invalidateQueries({ queryKey: ["knowledge-bases", user, workspace] });
      if (!active.current) return;
      setDeleteTarget(null);
      toast.success(text("libraryDeleted"));
    },
    onError: (error) => { if (active.current) toast.error(error.message); },
  });
  const create = useMutation({ mutationFn: () => createKnowledgeLibrary(workspace, name.trim(), description.trim()),
    onSuccess: (library) => { void cache.invalidateQueries({ queryKey: ["knowledge-bases", user, workspace] }); if (!active.current) return; toast.success(text("created")); navigate(`/knowledge-base/${encodeURIComponent(library.id)}`); },
    onError: (error) => { if (active.current) toast.error(error.message); },
  });
  const libraries = useMemo(() => (query.data ?? []).filter((item) => item.name.toLocaleLowerCase().includes(filter.trim().toLocaleLowerCase())), [query.data, filter]);

  return <>
    <PageHeader title={text("title")} actions={<>
      <Button variant="outline" disabled={!workspace || query.isFetching} onClick={() => void query.refetch()}>{text("refresh")}</Button>
      <Button disabled={!workspace} onClick={() => setOpen(true)}>{text("create")}</Button>
    </>} />
    {!workspace ? <p role="status">{text("selectWorkspace")}</p> : <SurfaceCard>
      <TextInput type="search" aria-label={text("search")} placeholder={text("search")} value={filter} onChange={(event) => setFilter(event.target.value)} className="mb-4 max-w-sm" />
      {query.error ? <p role="alert" className="text-[var(--color-error)]">{query.error.message}</p> : null}
      <Table><TableHeader><TableRow>
        <TableHead>{text("name")}</TableHead><TableHead>{text("description")}</TableHead><TableHead>{text("documents")}</TableHead><TableHead>{text("authorizedAgents")}</TableHead><TableHead className="w-16 text-right">{text("actions")}</TableHead>
      </TableRow></TableHeader><TableBody>
        {libraries.map((item) => <TableRow key={item.id} tabIndex={0} className="cursor-pointer focus-visible:outline-2 focus-visible:outline-[var(--color-focus-ring)]" onClick={() => navigate(`/knowledge-base/${encodeURIComponent(item.id)}`)} onKeyDown={(event) => { if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); navigate(`/knowledge-base/${encodeURIComponent(item.id)}`); } }}>
          <TableCell className="font-medium">{item.name}</TableCell>
          <TableCell className="max-w-md whitespace-normal">{item.description || "—"}</TableCell>
          <TableCell>{new Intl.NumberFormat(i18n.resolvedLanguage).format(item.documentCount)}</TableCell>
          <TableCell>{new Intl.NumberFormat(i18n.resolvedLanguage).format(item.authorizedAgentCount)}</TableCell>
          <TableCell className="text-right" onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}><DropdownMenu>
            <DropdownMenuTrigger render={<Button variant="ghost" size="icon" disabled={deletion.isPending} aria-label={t("knowledgeLibrary.libraryActions", { name: item.name })}><MoreHorizontal size={16} aria-hidden="true" /></Button>} />
            <DropdownMenuContent align="end" onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}>
              {item.documentCount > 0 ? <Tooltip><TooltipTrigger render={<span className="block cursor-not-allowed" />}><DropdownMenuItem disabled variant="destructive" className="data-disabled:opacity-35">{text("deleteAction")}</DropdownMenuItem></TooltipTrigger><TooltipContent side="left">{text("libraryDeleteRequiresEmpty")}</TooltipContent></Tooltip> : <DropdownMenuItem variant="destructive" onClick={(event) => { event.stopPropagation(); setDeleteTarget(item); }}>{text("deleteAction")}</DropdownMenuItem>}
            </DropdownMenuContent>
          </DropdownMenu></TableCell>
        </TableRow>)}
      </TableBody></Table>
      {!libraries.length && !query.error ? <p role="status" className="py-8 text-center text-[var(--color-on-surface-variant)]">{text(query.isLoading ? "loading" : "empty")}</p> : null}
    </SurfaceCard>}
    <ConfirmDialog open={Boolean(deleteTarget)} title={text("libraryDeleteTitle")} description={t("knowledgeLibrary.libraryDeleteDescription", { name: deleteTarget?.name ?? "" })} cancelText={text("cancel")} confirmText={text("deleteAction")} pending={deletion.isPending} onCancel={() => setDeleteTarget(null)} onConfirm={() => { if (deleteTarget) deletion.mutate(deleteTarget); }} />
    <Dialog open={open} onOpenChange={(value) => { if (!create.isPending) setOpen(value); }}>
      <DialogContent closeLabel={text("cancel")}><DialogHeader><DialogTitle>{text("create")}</DialogTitle></DialogHeader>
        <form onSubmit={(event) => { event.preventDefault(); if (name.trim()) create.mutate(); }} className="grid gap-4">
          <Field label={text("name")} required><TextInput value={name} onChange={(event) => setName(event.target.value)} maxLength={120} required disabled={create.isPending} /></Field>
          <Field label={text("description")}><TextArea value={description} onChange={(event) => setDescription(event.target.value)} maxLength={2000} disabled={create.isPending} /></Field>
          <DialogFooter><Button type="button" variant="outline" disabled={create.isPending} onClick={() => setOpen(false)}>{text("cancel")}</Button><Button type="submit" disabled={!name.trim() || create.isPending}>{text(create.isPending ? "saving" : "create")}</Button></DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </>;
}
