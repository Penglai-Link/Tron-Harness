import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import { ExternalLink, FolderDown, Unlink } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useToast } from "@/components/ui/toast";
import { useChatSessionGroupList } from "@/domain/chat/hooks";
import { useCreateChatTab } from "@/shared/layout/chat/hooks/useCreateChatTab";
import { getProject } from "../api/projectApi";
import { getTopicMemo, removeTopicReference, saveTopicNotes, type TopicMemo, type TopicReference } from "../api/topicApi";
import type { ProjectRecord } from "../model/ProjectModel";
import { TopicReferencePicker } from "../components/TopicReferencePicker";
import { TopicCheckoutDialog } from "../components/TopicCheckoutDialog";

export function TopicPage() {
  const { projectCode = "" } = useParams();
  const { t } = useTranslation("shell");
  const toast = useToast();
  const [project, setProject] = useState<ProjectRecord | null>(null);
  const [memo, setMemo] = useState<TopicMemo | null>(null);
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [checkoutReference, setCheckoutReference] = useState<TopicReference | null>(null);
  const [revision, setRevision] = useState(0);
  const [sessionsRevision, setSessionsRevision] = useState(0);
  const [referencesChanged, setReferencesChanged] = useState(false);
  const currentNotes = useRef(notes);
  const currentMemo = useRef(memo);
  currentNotes.current = notes;
  currentMemo.current = memo;
  const refresh = useCallback(() => { setRevision((value) => value + 1); setSessionsRevision((value) => value + 1); }, []);
  const workspaceCode = project?.workspaceCode ?? "";
  const groups = useChatSessionGroupList(workspaceCode, sessionsRevision);
  const { create } = useCreateChatTab(workspaceCode, projectCode);
  const sessions = useMemo(() => groups.flatMap((group) => group.sessions).filter((session) => session.projectCode === projectCode), [groups, projectCode]);

  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    Promise.all([getProject(projectCode, controller.signal), getTopicMemo(projectCode, controller.signal)]).then(([nextProject, nextMemo]) => {
      if (controller.signal.aborted) return;
      setProject(nextProject);
      setMemo(nextMemo);
      setNotes(nextMemo.notes);
      setReferencesChanged(false);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : t("topic.loadFailed"));
    });
    return () => controller.abort();
  }, [projectCode, revision, t]);

  useEffect(() => {
    const changed = (event: Event) => {
      const detail = (event as CustomEvent<{ oldProjectCode?: string | null; projectCode?: string | null }>).detail;
      if (detail?.oldProjectCode !== projectCode && detail?.projectCode !== projectCode) return;
      setSessionsRevision((value) => value + 1);
      // Preserve unsaved notes and their original revision; the next save must
      // detect concurrent changes instead of overwriting them silently.
      if (currentNotes.current !== currentMemo.current?.notes) { setReferencesChanged(true); return; }
      const originalRevision = currentMemo.current?.revision;
      void getTopicMemo(projectCode).then((next) => {
        if (currentNotes.current !== currentMemo.current?.notes || currentMemo.current?.revision !== originalRevision) { setReferencesChanged(true); return; }
        setMemo(next); setNotes(next.notes); setReferencesChanged(false);
      }).catch(() => setReferencesChanged(true));
    };
    window.addEventListener("tron:topic-location-changed", changed);
    return () => window.removeEventListener("tron:topic-location-changed", changed);
  }, [projectCode]);

  async function save() {
    if (!memo || pending) return;
    setPending(true);
    try { const next = await saveTopicNotes(projectCode, memo.revision, notes); setMemo(next); setNotes(next.notes); }
    catch (reason) { toast.error(reason instanceof Error ? reason.message : t("topic.saveFailed")); }
    finally { setPending(false); }
  }

  async function remove(referenceCode: string) {
    if (!memo || pending) return;
    setPending(true);
    try { setMemo(await removeTopicReference(projectCode, referenceCode, memo.revision)); }
    catch (reason) { toast.error(reason instanceof Error ? reason.message : t("topic.saveFailed")); }
    finally { setPending(false); }
  }

  if (error) return <div className="p-6"><p role="alert">{error}</p><Button variant="outline" onClick={refresh}>{t("topic.retry")}</Button></div>;
  if (!project || !memo) return <div className="p-6">{t("chatSidebar.loading")}</div>;

  return (
    <main className="mx-auto flex w-full max-w-4xl flex-col gap-8 p-6">
      <header className="flex flex-wrap items-center justify-between gap-3"><h1 className="text-xl font-semibold">{project.name}</h1><Button onClick={() => create()}>{t("chatSidebar.newChat")}</Button></header>
      <section className="space-y-3" aria-labelledby="topic-notes-title">
        <h2 id="topic-notes-title" className="font-medium">{t("topic.notes")}</h2>
        <Textarea aria-label={t("topic.notes")} value={notes} onChange={(event) => setNotes(event.target.value)} rows={5} placeholder={t("topic.notesPlaceholder")} />
        <div className="flex gap-2"><Button disabled={pending || notes === memo.notes} onClick={() => void save()}>{t("topic.saveNotes")}</Button><Button variant="ghost" disabled={pending} onClick={refresh}>{t("topic.refresh")}</Button></div>
      </section>
      <section className="space-y-3" aria-labelledby="topic-references-title">
        <div className="flex items-center justify-between gap-3"><h2 id="topic-references-title" className="font-medium">{t("topic.references")}</h2><Button variant="outline" disabled={pending} onClick={() => setPickerOpen(true)}>{t("topic.addReference")}</Button></div>
        {referencesChanged ? <p role="status" className="text-sm text-muted-foreground">{t("topic.referencesChanged")}</p> : null}
        {memo.references.length === 0 ? <p className="text-sm text-muted-foreground">{t("topic.noReferences")}</p> : <ul className="divide-y divide-border rounded border border-border">
          {memo.references.map((reference) => <li key={reference.referenceCode} className="flex flex-wrap items-center gap-3 px-3 py-3">
            <span className="w-24 shrink-0 text-xs text-muted-foreground">{t(`topic.kind.${reference.kind}`)}</span>
            <div className="min-w-0 flex-1"><div className="truncate">{reference.label || t("topic.unavailable")}</div>{reference.directoryPath ? <div className="truncate text-xs text-muted-foreground">{reference.directoryPath}</div> : null}</div>
            {!reference.available && reference.label ? <span className="text-xs text-muted-foreground">{t("topic.unavailable")}</span> : null}
            <div className="ml-auto flex shrink-0 items-center gap-2">
              {reference.url ? <Button size="sm" variant="outline" render={<a href={reference.url} target="_blank" rel="noreferrer" />}><ExternalLink size={14} />{t("topic.open")}</Button> : null}
              {reference.kind === "Repository" && reference.available ? <Button size="sm" variant="secondary" disabled={pending || notes !== memo.notes} onClick={() => setCheckoutReference(reference)}><FolderDown size={14} />{t("topic.checkout.prepare")}</Button> : null}
              <Button size="icon-sm" variant="ghost" className="text-destructive hover:text-destructive" title={t("topic.remove")} aria-label={`${t("topic.remove")} ${reference.label}`} disabled={pending} onClick={() => void remove(reference.referenceCode)}><Unlink size={14} /></Button>
            </div>
          </li>)}
        </ul>}
      </section>
      <section className="space-y-3" aria-labelledby="topic-sessions-title"><h2 id="topic-sessions-title" className="font-medium">{t("topic.sessions")}</h2>
        {sessions.length ? <ul className="space-y-1">{sessions.map((session) => <li key={session.sessionCode}><Link className="text-sm underline" to={`/chats?${new URLSearchParams({ sessionCode: session.sessionCode, workspaceCode })}`}>{session.title}</Link></li>)}</ul> : <p className="text-sm text-muted-foreground">{t("chatSidebar.noSessions")}</p>}
      </section>
      <TopicReferencePicker open={pickerOpen} onOpenChange={setPickerOpen} projectCode={projectCode} workspaceCode={workspaceCode} revision={memo.revision} onAdded={setMemo} />
      <TopicCheckoutDialog projectCode={projectCode} workspaceCode={workspaceCode} reference={checkoutReference} onClose={() => setCheckoutReference(null)} onPrepared={(next) => {
        if (currentNotes.current !== currentMemo.current?.notes) { setReferencesChanged(true); return; }
        setMemo(next); setNotes(next.notes); setReferencesChanged(false);
      }} />
    </main>
  );
}
