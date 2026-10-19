import { RagSettingsFields } from "@/domain/rag/components/RagSettingsFields";
import { defaultRagSettings, canSaveRagSettings } from "@/domain/rag/model/ragSettings";
import { MoreHorizontal } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { PageHeader, SurfaceCard } from "@/shared";
import { Field, TextInput, TextArea } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useToast } from "@/components/ui/toast";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { createKnowledgeLibrary, deleteKnowledgeLibrary, listPlatformKnowledgeLibraries, type PlatformKnowledgeLibrarySummary } from "../api/knowledgeBaseApi";
import { KnowledgeLibrarySettingsDialog } from "../components/KnowledgeLibrarySettingsDialog";
import { KnowledgeGrantsDialog } from "../components/KnowledgeGrantsDialog";
import { DocumentParsingPreferenceField } from "../components/DocumentParsingPreferenceField";
import type { DocumentParsePreference } from "../model/KnowledgeLibraryModel";
import { useKnowledgeUser, useKnowledgeViewActive } from "../hooks/useKnowledgeLibraries";

export function KnowledgeBasePage() {
  const user = useKnowledgeUser();
  // Account changes must discard unfinished catalog drafts.
  return <KnowledgeCatalog key={user} />;
}

function KnowledgeCatalog() {
  const { t, i18n } = useTranslation("pages");
  const active = useKnowledgeViewActive();
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const navigate = useNavigate();
  const toast = useToast();
  const user = useKnowledgeUser();
  const cache = useQueryClient();
  const query = useQuery({ queryKey: ["platform-knowledge-bases", user], enabled: Boolean(user), queryFn: ({ signal }) => listPlatformKnowledgeLibraries( signal) });
  const [agentAssignment, setAgentAssignment] = useState<PlatformKnowledgeLibrarySummary | null>(null);
  const refresh = () => cache.invalidateQueries({ queryKey: ["platform-knowledge-bases", user] });
  const detailPath = (library: { id: string }) => `/settings/knowledge-base/${encodeURIComponent(library.id)}`;
  const [settingsLibraryId, setSettingsLibraryId] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [indexSettings, setIndexSettings] = useState(defaultRagSettings);
  const [parsePreference, setParsePreference] = useState<DocumentParsePreference>("Auto");
  const [deleteTarget, setDeleteTarget] = useState<PlatformKnowledgeLibrarySummary | null>(null);
  const deletion = useMutation({
    mutationFn: (library: PlatformKnowledgeLibrarySummary) => deleteKnowledgeLibrary( library.id),
    onSuccess: async (_, library) => {
      cache.removeQueries({ queryKey: ["knowledge-base", user, library.id] });
      await refresh();
      if (!active.current) return;
      setDeleteTarget(null);
      toast.success(text("libraryDeleted"));
    },
    onError: (error) => { if (active.current) toast.error(error.message); },
  });
  const create = useMutation({ mutationFn: () => createKnowledgeLibrary(name.trim(), description.trim(), indexSettings, parsePreference),
    onSuccess: (library) => { void refresh(); void cache.invalidateQueries({ queryKey: ["rag-corpora", user] }); if (!active.current) return; setOpen(false); toast.success(text("created")); navigate(detailPath(library)); },
    onError: (error) => { if (active.current) toast.error(error.message); },
  });
  const libraries = useMemo(() => (query.data ?? []).filter((item) => item.name.toLocaleLowerCase().includes(filter.trim().toLocaleLowerCase())), [query.data, filter]);

  return <>
    <PageHeader title={text("title")} actions={<>
      <Button variant="outline" disabled={query.isFetching} onClick={() => void query.refetch()}>{text("refresh")}</Button>
      <Button disabled={!user} onClick={() => { setName(""); setDescription(""); setIndexSettings(defaultRagSettings()); setParsePreference("Auto"); setOpen(true); }}>{text("create")}</Button>
    </>} />
    <SurfaceCard>
      <div className="mb-4 flex flex-wrap gap-3">
      <TextInput type="search" aria-label={text("search")} placeholder={text("search")} value={filter} onChange={(event) => setFilter(event.target.value)} className="max-w-sm" /></div>
      {query.error ? <p role="alert" className="text-[var(--color-error)]">{query.error.message}</p> : null}
      <Table><TableHeader><TableRow>
        <TableHead>{text("name")}</TableHead><TableHead>{text("description")}</TableHead><TableHead>{text("documents")}</TableHead><TableHead>{text("authorizedAgentCount")}</TableHead><TableHead className="w-16 text-right">{text("actions")}</TableHead>
      </TableRow></TableHeader><TableBody>
        {libraries.map((item) => <TableRow key={item.id} tabIndex={0} className="cursor-pointer focus-visible:outline-2 focus-visible:outline-[var(--color-focus-ring)]" onClick={() => navigate(detailPath(item))} onKeyDown={(event) => { if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); navigate(detailPath(item)); } }}>
          <TableCell className="font-medium">{item.name}</TableCell>
          <TableCell className="max-w-md whitespace-normal">{item.description || "—"}</TableCell>
          <TableCell>{new Intl.NumberFormat(i18n.resolvedLanguage).format(item.documentCount)}</TableCell>
          <TableCell>{new Intl.NumberFormat(i18n.resolvedLanguage).format(item.authorizedAgentCount ?? 0)}</TableCell>
          <TableCell className="text-right" onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}><DropdownMenu>
            <DropdownMenuTrigger render={<Button variant="ghost" size="icon" disabled={deletion.isPending} aria-label={t("knowledgeLibrary.libraryActions", { name: item.name })}><MoreHorizontal size={16} aria-hidden="true" /></Button>} />
            <DropdownMenuContent align="end" onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}>
              <DropdownMenuItem onClick={() => setSettingsLibraryId(item.id)}>{text("librarySettings")}</DropdownMenuItem>
              <DropdownMenuItem onClick={() => setAgentAssignment(item)}>{text("grants")}</DropdownMenuItem>
              {item.documentCount > 0 ? <Tooltip><TooltipTrigger render={<span className="block cursor-not-allowed" />}><DropdownMenuItem disabled variant="destructive" className="data-disabled:opacity-35">{text("deleteAction")}</DropdownMenuItem></TooltipTrigger><TooltipContent side="left">{text("libraryDeleteRequiresEmpty")}</TooltipContent></Tooltip> : <DropdownMenuItem variant="destructive" onClick={(event) => { event.stopPropagation(); setDeleteTarget(item); }}>{text("deleteAction")}</DropdownMenuItem>}
            </DropdownMenuContent>
          </DropdownMenu></TableCell>
        </TableRow>)}
      </TableBody></Table>
      {!libraries.length && !query.error ? <p role="status" className="py-8 text-center text-[var(--color-on-surface-variant)]">{text(query.isLoading ? "loading" : "empty")}</p> : null}
    </SurfaceCard>
    {settingsLibraryId ? <KnowledgeLibrarySettingsDialog key={settingsLibraryId} libraryId={settingsLibraryId} onClose={() => setSettingsLibraryId(null)} /> : null}
    {agentAssignment ? <KnowledgeGrantsDialog libraryId={agentAssignment.id} onClose={() => setAgentAssignment(null)} onChanged={refresh} /> : null}
    <ConfirmDialog open={Boolean(deleteTarget)} title={text("libraryDeleteTitle")} description={t("knowledgeLibrary.libraryDeleteDescription", { name: deleteTarget?.name ?? "" })} cancelText={text("cancel")} confirmText={text("deleteAction")} pending={deletion.isPending} onCancel={() => setDeleteTarget(null)} onConfirm={() => { if (deleteTarget) deletion.mutate(deleteTarget); }} />
    <Dialog open={open} onOpenChange={(value) => { if (!create.isPending) setOpen(value); }}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-2xl" closeLabel={text("cancel")}><DialogHeader><DialogTitle>{text("create")}</DialogTitle></DialogHeader>
        <form onSubmit={(event) => { event.preventDefault(); if (name.trim() && canSaveRagSettings(indexSettings)) create.mutate(); }} className="grid gap-4">
          <Field label={text("name")} required><TextInput aria-label={text("name")} value={name} onChange={(event) => setName(event.target.value)} maxLength={120} required disabled={create.isPending} /></Field>
          <Field label={text("description")}><TextArea aria-label={text("description")} value={description} onChange={(event) => setDescription(event.target.value)} maxLength={2000} disabled={create.isPending} /></Field>
          <DocumentParsingPreferenceField value={parsePreference} onChange={setParsePreference} disabled={create.isPending} />
          <section aria-labelledby="knowledge-create-index-settings" className="grid gap-4 border-t pt-4">
            <h3 id="knowledge-create-index-settings" className="font-medium">{text("indexSettings")}</h3>
            <RagSettingsFields settings={indexSettings} setSettings={setIndexSettings} knowledgeBase disabled={create.isPending} idPrefix="knowledge-create-rag" className="sm:grid-cols-2" />
          </section>
          <DialogFooter><Button type="button" variant="outline" disabled={create.isPending} onClick={() => setOpen(false)}>{text("cancel")}</Button><Button type="submit" disabled={!name.trim() || !canSaveRagSettings(indexSettings) || create.isPending}>{text(create.isPending ? "saving" : "create")}</Button></DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </>;
}
