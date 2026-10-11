import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Field, TextArea, TextInput } from "@/components/forms";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { archiveDudeTask, createDudeTask, dudeQueryKeys, listDudeTasks, updateDudeTask, type DudeTask } from "../api/dudeApi";
function localDatetime(value: number | null) { if (value === null) return ""; const date = new Date(value); return new Date(value - date.getTimezoneOffset() * 60000).toISOString().slice(0, 16); }
export function DudeTasks({ dudeCode, active, onOpenSession, highlightedTask }: { dudeCode: string; active: boolean; onOpenSession: (sessionCode: string) => void; highlightedTask?: string | null }) {
  const { t, i18n } = useTranslation("pages"); const client = useQueryClient();
  const query = useQuery({ queryKey: dudeQueryKeys.tasks(dudeCode), queryFn: ({ signal }) => listDudeTasks(dudeCode, signal), enabled: active, refetchInterval: active ? 15000 : false });
  const [editor, setEditor] = useState<DudeTask | "new" | null>(null); const [description, setDescription] = useState(""); const [due, setDue] = useState(""); const [busy, setBusy] = useState(false); const [error, setError] = useState<string | null>(null);
  const formatDate = (value: number) => new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(value);
  function open(task: DudeTask | "new") { setEditor(task); setDescription(task === "new" ? "" : task.description ?? task.title); setDue(task === "new" ? "" : localDatetime(task.dueAt)); setError(null); }
  async function mutate(action: () => Promise<unknown>) { setBusy(true); setError(null); try { await action(); await client.invalidateQueries({ queryKey: dudeQueryKeys.tasks(dudeCode) }); setEditor(null); } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); } finally { setBusy(false); } }
  const payload = { description: description.trim(), dueAt: due ? new Date(due).getTime() : null };
  return <div className="space-y-4">
    <div className="flex justify-end"><Button size="sm" onClick={() => open("new")}>{t("dude.newTask")}</Button></div>
    {query.isPending ? <p role="status">{t("dude.loading")}</p> : null}{query.error ? <div role="alert">{query.error.message}<Button variant="outline" size="sm" onClick={() => void query.refetch()}>{t("dude.retry")}</Button></div> : null}
    {!editor && error ? <p role="alert" className="text-[var(--color-error)]">{error}</p> : null}
    <div className="divide-y divide-[var(--color-outline-variant)]">{query.data?.map(task => <article key={task.taskCode} data-task-code={task.taskCode} className={`flex flex-wrap items-start gap-3 py-3 ${highlightedTask === task.taskCode ? "bg-[var(--color-surface-container)]" : ""}`}>
      <div className="min-w-0 flex-1"><div className="flex items-center gap-2"><h3 className="break-words font-medium">{task.title}</h3><Badge variant="outline">{t(`dude.taskStatuses.${task.status}`, { defaultValue: task.status })}</Badge></div>{task.dueAt !== null ? <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">{t("dude.deadline")} · {formatDate(task.dueAt)}</p> : null}{task.plannedStartAt !== null ? <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">{t("dude.plannedStart")} · {formatDate(task.plannedStartAt)}</p> : null}{task.nextFollowupAt !== null ? <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">{t("dude.nextFollowup")} · {formatDate(task.nextFollowupAt)}</p> : null}</div>
      <div className="flex flex-wrap gap-2">{task.workSessionCode ? <Button size="sm" variant="outline" onClick={() => onOpenSession(task.workSessionCode!)}>{t("dude.viewWork")}</Button> : null}<Button size="sm" variant="ghost" disabled={busy} onClick={() => open(task)}>{t("dude.edit")}</Button><Button size="sm" variant="ghost" disabled={busy} onClick={() => void mutate(() => archiveDudeTask(task.taskCode))}>{t("dude.cancelTask")}</Button></div>
    </article>)}</div>
    {query.data?.length === 0 ? <p className="text-sm text-[var(--color-on-surface-variant)]">{t("dude.noTasks")}</p> : null}
    {editor ? <Dialog open disablePointerDismissal onOpenChange={(open, details) => { if (busy) details.cancel(); else if (!open) setEditor(null); }}><DialogContent closeLabel={t("dude.close")}><DialogHeader><DialogTitle>{t(editor === "new" ? "dude.newTask" : "dude.editTask")}</DialogTitle></DialogHeader><form className="space-y-4" onSubmit={event => { event.preventDefault(); if (!payload.description || !Number.isFinite(payload.dueAt ?? 0)) return; void mutate(() => editor === "new" ? createDudeTask(dudeCode, payload) : updateDudeTask(dudeCode, editor.taskCode, payload)); }}>
      <Field required label={t("dude.taskDescription")}><TextArea required aria-label={t("dude.taskDescription")} rows={4} disabled={busy} value={description} onChange={event => setDescription(event.target.value)} /></Field>
      <Field label={t("dude.deadlineOptional")} hint={t("dude.deadlineHelp")}><TextInput type="datetime-local" aria-label={t("dude.deadlineOptional")} disabled={busy} value={due} onChange={event => setDue(event.target.value)} /></Field>
      {error ? <p role="alert" className="text-[var(--color-error)]">{error}</p> : null}<DialogFooter><Button type="button" variant="outline" disabled={busy} onClick={() => setEditor(null)}>{t("dude.cancel")}</Button><Button type="submit" disabled={busy || !description.trim()}>{t(busy ? "dude.saving" : "dude.save")}</Button></DialogFooter>
    </form></DialogContent></Dialog> : null}
  </div>;
}
