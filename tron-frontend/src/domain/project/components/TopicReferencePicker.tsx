import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useToast } from "@/components/ui/toast";
import { useDirectories } from "@/domain/directory/hooks/useDirectories";
import { createResource, pageResources, type ResourceApiRecord } from "@/domain/resource/api";
import { suggestQuickResourceType } from "@/domain/resource/features/quick-upload/quickResourceUpload";
import { fetchTaskPage } from "@/domain/task/api/taskApi";
import type { TaskListItem } from "@/domain/task/model";
import { addTopicReference, type AddTopicReference, type TopicMemo, type TopicReferenceKind } from "../api/topicApi";
import { listConnections, searchRepositories, type ConnectorRecord, type ConnectorRepository } from "../api/connectorApi";

type Mode = TopicReferenceKind;

export function TopicReferencePicker({ open, onOpenChange, projectCode, workspaceCode, revision, onAdded }: {
  open: boolean; onOpenChange: (open: boolean) => void; projectCode: string; workspaceCode: string;
  revision: number; onAdded: (memo: TopicMemo) => void;
}) {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const { items: directories, refresh: refreshDirectories } = useDirectories(workspaceCode);
  const [mode, setMode] = useState<Mode>("Directory");
  const [query, setQuery] = useState("");
  const [resources, setResources] = useState<ResourceApiRecord[]>([]);
  const [tasks, setTasks] = useState<TaskListItem[]>([]);
  const [connections, setConnections] = useState<ConnectorRecord[]>([]);
  const [connectionCode, setConnectionCode] = useState("");
  const [repositories, setRepositories] = useState<ConnectorRepository[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [link, setLink] = useState("");
  const [linkLabel, setLinkLabel] = useState("");
  const [pending, setPending] = useState(false);
  useEffect(() => {
    if (!open) return;
    refreshDirectories();
    let live = true;
    listConnections().then((values) => {
      if (!live) return;
      setConnections(values);
      setConnectionCode((current) => values.some((item) => item.connectionCode === current) ? current : values[0]?.connectionCode ?? "");
    }).catch((reason: unknown) => { if (live) toast.error(reason instanceof Error ? reason.message : t("topic.loadFailed")); });
    return () => { live = false; };
  }, [open, workspaceCode, t, toast, refreshDirectories]);

  async function add(input: Omit<AddTopicReference, "revision">) {
    if (pending) return;
    setPending(true);
    try { onAdded(await addTopicReference(projectCode, { ...input, revision })); onOpenChange(false); }
    catch (reason) { toast.error(reason instanceof Error ? reason.message : t("topic.saveFailed")); }
    finally { setPending(false); }
  }

  async function search() {
    if (pending) return;
    setPending(true);
    try {
      if (mode === "Resource") {
        const result = await pageResources({ workspaceCode, kinds: ["File", "Image", "Pdf"], sourceKind: null, search: query.trim() || null, current: 1, size: 50 });
        setResources(result.records);
      } else if (mode === "Task") {
        const result = await fetchTaskPage({ workspaceCode, search: query.trim() || undefined, current: 1, size: 50 });
        setTasks(result.records);
      } else if (mode === "Repository" && connectionCode) {
        const result = await searchRepositories(connectionCode, query);
        setRepositories(result.records); setNextCursor(result.nextCursor);
      }
    } catch (reason) { toast.error(reason instanceof Error ? reason.message : t("topic.loadFailed")); }
    finally { setPending(false); }
  }

  async function moreRepositories() {
    if (!nextCursor || !connectionCode || pending) return;
    setPending(true);
    try {
      const result = await searchRepositories(connectionCode, query, nextCursor);
      setRepositories((current) => [...current, ...result.records]); setNextCursor(result.nextCursor);
    } catch (reason) { toast.error(reason instanceof Error ? reason.message : t("topic.loadFailed")); }
    finally { setPending(false); }
  }

  async function upload(file: File) {
    if (pending) return;
    setPending(true);
    try {
      const result = await createResource({ workspaceCode, resourceCode: `topic-${crypto.randomUUID()}`, kind: suggestQuickResourceType(file.name), name: file.name, description: "", allowInheritance: false, file });
      const memo = await addTopicReference(projectCode, { revision, kind: "Resource", targetCode: result.resourceCode, connectionCode: null, label: file.name, url: null });
      onAdded(memo); onOpenChange(false);
    } catch (reason) { toast.error(reason instanceof Error ? reason.message : t("topic.uploadFailed")); }
    finally { setPending(false); }
  }

  function selectMode(value: Mode) { setMode(value); setQuery(""); setRepositories([]); setNextCursor(null); setResources([]); setTasks([]); }

  return <Dialog open={open} onOpenChange={onOpenChange}><DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
    <DialogHeader><DialogTitle>{t("topic.addReference")}</DialogTitle></DialogHeader>
    <div className="flex flex-wrap gap-1">{(["Directory", "Repository", "Resource", "Task", "Link"] as Mode[]).map((value) => <Button key={value} size="sm" variant={mode === value ? "secondary" : "ghost"} onClick={() => selectMode(value)}>{t(`topic.kind.${value}`)}</Button>)}</div>
    {mode === "Directory" ? <div className="space-y-1">{directories.length ? directories.map((directory) => <Button key={directory.directoryCode} variant="ghost" className="h-auto w-full justify-start py-2 text-left" disabled={pending} onClick={() => void add({ kind: "Directory", targetCode: directory.directoryCode, connectionCode: null, label: directory.name, url: null })}><span className="min-w-0"><span className="block truncate">{directory.name}</span><span className="block truncate text-xs text-muted-foreground">{directory.path}</span></span></Button>) : <p className="text-sm text-muted-foreground">{t("topic.noDirectories")}</p>}</div> : null}
    {mode === "Link" ? <div className="space-y-3"><Field><FieldLabel htmlFor="topic-link">{t("topic.url")}</FieldLabel><Input id="topic-link" value={link} onChange={(event) => setLink(event.target.value)} placeholder="https://" /></Field><Field><FieldLabel htmlFor="topic-link-label">{t("topic.label")}</FieldLabel><Input id="topic-link-label" value={linkLabel} onChange={(event) => setLinkLabel(event.target.value)} /></Field><Button disabled={pending || !link.trim()} onClick={() => void add({ kind: "Link", targetCode: link.trim(), connectionCode: null, label: linkLabel.trim() || link.trim(), url: link.trim() })}>{t("topic.add")}</Button></div> : null}
    {mode === "Repository" ? <div className="space-y-3">
      <div className="flex flex-wrap gap-1">{connections.map((connection) => <Button key={connection.connectionCode} variant={connectionCode === connection.connectionCode ? "secondary" : "ghost"} size="sm" onClick={() => { setConnectionCode(connection.connectionCode); setRepositories([]); setNextCursor(null); }}>{connection.name}</Button>)}</div>
      <Link className="text-sm text-primary underline" to="/settings/code-repositories" onClick={() => onOpenChange(false)}>{t("topic.manageConnections")}</Link>
    </div> : null}
    {mode === "Resource" ? <Field><FieldLabel htmlFor="topic-upload">{t("topic.upload")}</FieldLabel><Input id="topic-upload" type="file" disabled={pending} onChange={(event) => { const file = event.target.files?.[0]; if (file) void upload(file); event.currentTarget.value = ""; }} /></Field> : null}
    {mode === "Resource" || mode === "Task" || mode === "Repository" ? <div className="flex gap-2"><Input aria-label={t("topic.search")} value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("topic.searchPlaceholder")} /><Button disabled={pending || (mode === "Repository" && !connectionCode)} onClick={() => void search()}>{t("topic.search")}</Button></div> : null}
    {mode === "Resource" ? <div className="space-y-1">{resources.map((resource) => <Button key={resource.resourceCode} variant="ghost" className="w-full justify-start" onClick={() => void add({ kind: "Resource", targetCode: resource.resourceCode, connectionCode: null, label: resource.name, url: null })}>{resource.name}</Button>)}</div> : null}
    {mode === "Task" ? <div className="space-y-1">{tasks.map((task) => <Button key={task.taskCode} variant="ghost" className="w-full justify-start" onClick={() => void add({ kind: "Task", targetCode: task.taskCode, connectionCode: null, label: task.title, url: null })}>{task.title}</Button>)}</div> : null}
    {mode === "Repository" ? <div className="space-y-1">{repositories.map((repository) => <Button key={repository.id} variant="ghost" className="w-full justify-start" onClick={() => void add({ kind: "Repository", targetCode: repository.id, connectionCode, label: repository.fullName, url: repository.webUrl })}>{repository.fullName}</Button>)}{nextCursor ? <Button variant="ghost" onClick={() => void moreRepositories()} disabled={pending}>{t("topic.more")}</Button> : null}</div> : null}
    <DialogFooter><Button variant="outline" onClick={() => onOpenChange(false)}>{t("topic.close")}</Button></DialogFooter>
  </DialogContent></Dialog>;
}
