import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { useMutation } from "@tanstack/react-query";
import { PageHeader, SurfaceCard } from "@/shared";
import { Field, TextInput, TextArea } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useToast } from "@/components/ui/toast";
import { createKnowledgeLibrary } from "../api/knowledgeBaseApi";
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
  const query = useKnowledgeLibraries(workspace);
  const [filter, setFilter] = useState("");
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const create = useMutation({ mutationFn: () => createKnowledgeLibrary(workspace, name.trim(), description.trim()),
    onSuccess: (library) => { if (!active.current) return; toast.success(text("created")); navigate(`/knowledge-base/${encodeURIComponent(library.id)}`); },
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
        <TableHead>{text("name")}</TableHead><TableHead>{text("description")}</TableHead><TableHead>{text("documents")}</TableHead>
      </TableRow></TableHeader><TableBody>
        {libraries.map((item) => <TableRow key={item.id}>
          <TableCell><Button variant="link" onClick={() => navigate(`/knowledge-base/${encodeURIComponent(item.id)}`)}>{item.name}</Button></TableCell>
          <TableCell className="max-w-md whitespace-normal">{item.description || "—"}</TableCell>
          <TableCell>{new Intl.NumberFormat(i18n.resolvedLanguage).format(item.documentCount)}</TableCell>
        </TableRow>)}
      </TableBody></Table>
      {!libraries.length && !query.error ? <p role="status" className="py-8 text-center text-[var(--color-on-surface-variant)]">{text(query.isLoading ? "loading" : "empty")}</p> : null}
    </SurfaceCard>}
    <details className="mt-4 text-sm"><summary className="cursor-pointer text-[var(--color-on-surface-variant)]">{text("advanced")}</summary><Button className="mt-2" variant="outline" onClick={() => navigate("/rag")}>{text("advancedRag")}</Button></details>
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
