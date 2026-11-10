import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useToast } from "@/components/ui/toast";
import { useDirectories } from "@/domain/directory/hooks/useDirectories";
import { createResource } from "@/domain/resource/api";
import { suggestQuickResourceType } from "@/domain/resource/features/quick-upload/quickResourceUpload";
import { fetchTaskPage } from "@/domain/task/api/taskApi";
import type { TaskListItem } from "@/domain/task/model";
import { addTopicReference, type AddTopicReference, type TopicMemo, type TopicReferenceKind } from "../api/topicApi";
import { listConnections, searchRepositories, type ConnectorRecord, type ConnectorRepository } from "../api/connectorApi";

import { TopicFileResourcePicker } from "./TopicFileResourcePicker";

type Mode = TopicReferenceKind;

function normalizeTopicLink(value: string): string | null {
  const address = value.trim();
  if (!address || /[\s\\]/u.test(address) || /^\/(?!\/)|^\.\.?\//.test(address)) return null;
  // Only infer HTTPS for a host, never reinterpret an explicit non-web scheme.
  const hasScheme = /^[a-z][a-z\d+.-]*:/i.test(address);
  const candidate = address.startsWith("//") ? `https:${address}` : hasScheme ? address : `https://${address}`;
  try {
    const url = new URL(candidate);
    if (!["http:", "https:"].includes(url.protocol) || !url.hostname || url.username || url.password
      || new TextEncoder().encode(url.href).length > 8192) return null;
    return url.href;
  } catch {
    return null;
  }
}

export function TopicReferencePicker({ open, onOpenChange, projectCode, workspaceCode, revision, onAdded }: {
  open: boolean; onOpenChange: (open: boolean) => void; projectCode: string; workspaceCode: string;
  revision: number; onAdded: (memo: TopicMemo) => void;
}) {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const { items: directories, refresh: refreshDirectories } = useDirectories(workspaceCode);
  const [mode, setMode] = useState<Mode>("Directory");
  const [query, setQuery] = useState("");
  const [tasks, setTasks] = useState<TaskListItem[]>([]);
  const [connections, setConnections] = useState<ConnectorRecord[]>([]);
  const [connectionCode, setConnectionCode] = useState("");
  const [repositories, setRepositories] = useState<ConnectorRepository[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [link, setLink] = useState("");
  const [linkLabel, setLinkLabel] = useState("");
  const [linkInvalid, setLinkInvalid] = useState(false);
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

  function addLink() {
    if (pending) return;
    const url = normalizeTopicLink(link);
    if (!url) {
      setLinkInvalid(true);
      toast.error(t("topic.invalidLink"));
      return;
    }
    setLinkInvalid(false);
    setLink(url);
    const fallbackLabel = [...url].length <= 200 ? url : new URL(url).hostname.slice(0, 200);
    void add({ kind: "Link", targetCode: url, connectionCode: null, label: linkLabel.trim() || fallbackLabel, url });
  }

  async function search() {
    if (pending) return;
    setPending(true);
    try {
      if (mode === "Task") {
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

  function selectMode(value: Mode) { setMode(value); setQuery(""); setRepositories([]); setNextCursor(null); setTasks([]); }

  return <Dialog open={open} onOpenChange={(value) => { if (!pending) onOpenChange(value); }}><DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
    <DialogHeader><DialogTitle>{t("topic.addReference")}</DialogTitle></DialogHeader>
    <div className="flex flex-wrap gap-1">{(["Directory", "Repository", "Resource", "Task", "Link"] as Mode[]).map((value) => <Button key={value} size="sm" disabled={pending} variant={mode === value ? "secondary" : "ghost"} onClick={() => selectMode(value)}>{t(`topic.kind.${value}`)}</Button>)}</div>
    {mode === "Directory" ? <div className="space-y-1">{directories.length ? directories.map((directory) => <Button key={directory.directoryCode} variant="ghost" className="h-auto w-full justify-start py-2 text-left" disabled={pending} onClick={() => void add({ kind: "Directory", targetCode: directory.directoryCode, connectionCode: null, label: directory.name, url: null })}><span className="min-w-0"><span className="block truncate">{directory.name}</span><span className="block truncate text-xs text-muted-foreground">{directory.path}</span></span></Button>) : <p className="text-sm text-muted-foreground">{t("topic.noDirectories")}</p>}</div> : null}
    {mode === "Link" ? <div className="space-y-3">
      <Field>
        <FieldLabel htmlFor="topic-link">{t("topic.url")}</FieldLabel>
        <Input id="topic-link" value={link} aria-invalid={linkInvalid || undefined} aria-describedby="topic-link-hint" disabled={pending}
          onChange={(event) => { setLink(event.target.value); setLinkInvalid(false); }} placeholder="https://example.com" />
        <p id="topic-link-hint" className="text-xs text-muted-foreground">{t("topic.linkHint")}</p>
      </Field>
      <Field><FieldLabel htmlFor="topic-link-label">{t("topic.label")}</FieldLabel><Input id="topic-link-label" value={linkLabel} maxLength={200} disabled={pending} onChange={(event) => setLinkLabel(event.target.value)} /></Field>
      <Button data-dialog-primary disabled={pending || !link.trim()} onClick={addLink}>{t("topic.add")}</Button>
    </div> : null}
    {mode === "Repository" ? <div className="space-y-3">
      <div className="flex flex-wrap gap-1">{connections.map((connection) => <Button key={connection.connectionCode} variant={connectionCode === connection.connectionCode ? "secondary" : "ghost"} size="sm" onClick={() => { setConnectionCode(connection.connectionCode); setRepositories([]); setNextCursor(null); }}>{connection.name}</Button>)}</div>
      <Link className="text-sm text-primary underline" to="/settings/code-repositories" onClick={() => onOpenChange(false)}>{t("topic.manageConnections")}</Link>
    </div> : null}
    {mode === "Resource" && open ? <TopicFileResourcePicker key={workspaceCode} workspaceCode={workspaceCode} pending={pending}
      onSelect={(resource) => void add({ kind: "Resource", targetCode: resource.resourceCode, connectionCode: null, label: resource.name, url: null })}
      onUpload={upload} /> : null}
    {mode === "Task" || mode === "Repository" ? <div className="flex gap-2"><Input aria-label={t("topic.search")} value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("topic.searchPlaceholder")} /><Button data-dialog-primary disabled={pending || (mode === "Repository" && !connectionCode)} onClick={() => void search()}>{t("topic.search")}</Button></div> : null}
    {mode === "Task" ? <div className="space-y-1">{tasks.map((task) => <Button key={task.taskCode} variant="ghost" className="w-full justify-start" onClick={() => void add({ kind: "Task", targetCode: task.taskCode, connectionCode: null, label: task.title, url: null })}>{task.title}</Button>)}</div> : null}
    {mode === "Repository" ? <div className="space-y-1">{repositories.map((repository) => <Button key={repository.id} variant="ghost" className="w-full justify-start" onClick={() => void add({ kind: "Repository", targetCode: repository.id, connectionCode, label: repository.fullName, url: repository.webUrl })}>{repository.fullName}</Button>)}{nextCursor ? <Button variant="ghost" onClick={() => void moreRepositories()} disabled={pending}>{t("topic.more")}</Button> : null}</div> : null}
    <DialogFooter><Button variant="outline" disabled={pending} onClick={() => onOpenChange(false)}>{t("topic.close")}</Button></DialogFooter>
  </DialogContent></Dialog>;
}
