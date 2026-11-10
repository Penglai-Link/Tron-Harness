import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import { ExternalLink, FolderDown, Unlink } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useToast } from "@/components/ui/toast";
import { useChatSessionGroupList } from "@/domain/chat/hooks";
import { useCreateChatTab } from "@/shared/layout/chat/hooks/useCreateChatTab";
import { getProject } from "../api/projectApi";
import { removeTopicReference, type TopicReference } from "../api/topicApi";
import type { ProjectRecord } from "../model/ProjectModel";
import { TopicReferencePicker } from "../components/TopicReferencePicker";
import { TopicCheckoutDialog } from "../components/TopicCheckoutDialog";
import { useTopicNotes } from "../hooks/useTopicNotes";
import { useAuthStore } from "@/store/authStore";

export function TopicPage() {
  const { projectCode = "" } = useParams();
  const userCode = useAuthStore((state) => state.session?.user.userCode ?? "");
  return <TopicDetail key={`${userCode}:${projectCode}`} projectCode={projectCode} userCode={userCode} />;
}

function TopicDetail({ projectCode, userCode }: { projectCode: string; userCode: string }) {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const [project, setProject] = useState<ProjectRecord | null>(null);
  const { memo, notes, phase, loadError, editor } = useTopicNotes(projectCode, userCode);
  const [error, setError] = useState<unknown>(null);
  const [pending, setPending] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [checkoutReference, setCheckoutReference] = useState<TopicReference | null>(null);
  const [revision, setRevision] = useState(0);
  const [sessionsRevision, setSessionsRevision] = useState(0);
  const refresh = useCallback(() => {
    setRevision((value) => value + 1);
    setSessionsRevision((value) => value + 1);
    void editor.refresh();
  }, [editor]);
  const workspaceCode = project?.workspaceCode ?? "";
  const groups = useChatSessionGroupList(workspaceCode, sessionsRevision);
  const { create } = useCreateChatTab(workspaceCode, projectCode);
  const sessions = useMemo(() => groups.flatMap((group) => group.sessions).filter((session) => session.projectCode === projectCode), [groups, projectCode]);

  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    getProject(projectCode, controller.signal).then((nextProject) => {
      if (!controller.signal.aborted) setProject(nextProject);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason);
    });
    return () => controller.abort();
  }, [projectCode, revision]);

  useEffect(() => {
    const changed = (event: Event) => {
      const detail = (event as CustomEvent<{ oldProjectCode?: string | null; projectCode?: string | null }>).detail;
      if (detail?.oldProjectCode !== projectCode && detail?.projectCode !== projectCode) return;
      setSessionsRevision((value) => value + 1);
      void editor.refresh();
    };
    window.addEventListener("tron:topic-location-changed", changed);
    return () => window.removeEventListener("tron:topic-location-changed", changed);
  }, [projectCode, editor]);

  async function openPicker() {
    if (!pending && await editor.flush()) setPickerOpen(true);
  }

  async function openCheckout(reference: TopicReference) {
    if (!pending && await editor.flush()) setCheckoutReference(reference);
  }

  async function openChat() {
    if (!pending && await editor.flush()) create();
  }

  async function remove(referenceCode: string) {
    if (!memo || pending) return;
    setPending(true);
    try {
      if (!await editor.flush()) return;
      const current = editor.getMemo();
      if (current) editor.acceptMemo(await removeTopicReference(projectCode, referenceCode, current.revision));
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : t("topic.saveFailed"));
      void editor.refresh();
    } finally { setPending(false); }
  }

  const pageError = error ?? loadError;
  const busy = pending;
  if (pageError) return <div className="p-6"><p role="alert">{pageError instanceof Error ? pageError.message : t("topic.loadFailed")}</p><Button variant="outline" onClick={refresh}>{t("topic.retry")}</Button></div>;
  if (!project || !memo) return <div className="p-6">{t("chatSidebar.loading")}</div>;

  return (
    <main className="mx-auto flex w-full max-w-4xl flex-col gap-8 p-6">
      <header className="flex flex-wrap items-center justify-between gap-3"><h1 className="text-xl font-semibold">{project.name}</h1><Button disabled={busy} onClick={() => void openChat()}>{t("chatSidebar.newChat")}</Button></header>
      <section className="space-y-3" aria-labelledby="topic-notes-title">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 id="topic-notes-title" className="font-medium">{t("topic.notes")}</h2>
          <span id="topic-notes-save-status" role="status" aria-live="polite" className="text-xs text-muted-foreground">{t(`topic.autosave.${phase}`)}</span>
        </div>
        <Textarea aria-label={t("topic.notes")} aria-describedby="topic-notes-save-status" value={notes}
          className="field-sizing-fixed h-[clamp(18rem,45dvh,40rem)] min-h-48 resize-y" rows={14} maxLength={65536} disabled={pending}
          onChange={(event) => editor.setNotes(event.target.value)} onBlur={() => void editor.flush()} placeholder={t("topic.notesPlaceholder")} />
        {phase === "error" ? <Button variant="ghost" size="sm" onClick={() => void editor.retry()}>{t("topic.retry")}</Button> : null}
        {phase === "conflict" ? <div className="space-y-3 rounded border border-border p-3 text-sm">
          <p>{t("topic.autosave.conflictHelp")}</p>
          <details><summary className="cursor-pointer">{t("topic.autosave.viewLatest")}</summary><pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-words font-sans">{memo.notes || t("topic.autosave.emptyLatest")}</pre></details>
          <div className="flex flex-wrap gap-2"><Button size="sm" variant="outline" onClick={editor.useLatest}>{t("topic.autosave.useLatest")}</Button><Button size="sm" onClick={() => void editor.keepMine()}>{t("topic.autosave.keepMine")}</Button></div>
        </div> : null}
      </section>
      <section className="space-y-3" aria-labelledby="topic-references-title">
        <div className="flex items-center justify-between gap-3"><h2 id="topic-references-title" className="font-medium">{t("topic.references")}</h2><Button variant="outline" disabled={busy} onClick={() => void openPicker()}>{t("topic.addReference")}</Button></div>
        {memo.references.length === 0 ? <p className="text-sm text-muted-foreground">{t("topic.noReferences")}</p> : <ul className="divide-y divide-border rounded border border-border">
          {memo.references.map((reference) => <li key={reference.referenceCode} className="flex flex-wrap items-center gap-3 px-3 py-3">
            <span className="w-24 shrink-0 text-xs text-muted-foreground">{t(`topic.kind.${reference.kind}`)}</span>
            <div className="min-w-0 flex-1"><div className="truncate">{reference.label || t("topic.unavailable")}</div>{reference.directoryPath ? <div className="truncate text-xs text-muted-foreground">{reference.directoryPath}</div> : null}</div>
            {!reference.available && reference.label ? <span className="text-xs text-muted-foreground">{t("topic.unavailable")}</span> : null}
            <div className="ml-auto flex shrink-0 items-center gap-2">
              {reference.url ? <Button size="sm" variant="outline" render={<a href={reference.url} target="_blank" rel="noreferrer" />}><ExternalLink size={14} />{t("topic.open")}</Button> : null}
              {reference.kind === "Repository" && reference.available ? <Button size="sm" variant="secondary" disabled={busy} onClick={() => void openCheckout(reference)}><FolderDown size={14} />{t("topic.checkout.prepare")}</Button> : null}
              <Button size="icon-sm" variant="ghost" className="text-destructive hover:text-destructive" title={t("topic.remove")} aria-label={`${t("topic.remove")} ${reference.label}`} disabled={busy} onClick={() => void remove(reference.referenceCode)}><Unlink size={14} /></Button>
            </div>
          </li>)}
        </ul>}
      </section>
      <section className="space-y-3" aria-labelledby="topic-sessions-title"><h2 id="topic-sessions-title" className="font-medium">{t("topic.sessions")}</h2>
        {sessions.length ? <ul className="space-y-1">{sessions.map((session) => <li key={session.sessionCode}><Link className="text-sm underline" to={`/chats?${new URLSearchParams({ sessionCode: session.sessionCode, workspaceCode })}`}>{session.title}</Link></li>)}</ul> : <p className="text-sm text-muted-foreground">{t("chatSidebar.noSessions")}</p>}
      </section>
      <TopicReferencePicker open={pickerOpen} onOpenChange={setPickerOpen} projectCode={projectCode} workspaceCode={workspaceCode} revision={memo.revision} onAdded={editor.acceptMemo} />
      <TopicCheckoutDialog projectCode={projectCode} workspaceCode={workspaceCode} reference={checkoutReference} onClose={() => setCheckoutReference(null)} onPrepared={editor.acceptMemo} />
    </main>
  );
}
