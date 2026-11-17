import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { CalendarClock, CircleX, FileClock, Pencil, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Field, TextArea, TextInput } from "@/components/forms";
import { Badge } from "@/components/ui/badge";
import { MarkdownContent } from "@/components/markdown-content";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  cancelAssistantTask,
  createAssistantTask,
  assistantQueryKeys,
  getAssistantTask,
  listAssistantTasks,
  updateAssistantTask,
  type AssistantTask,
  type AssistantTaskDetail,
} from "../api/assistantApi";

function localDatetime(value: number | null) {
  if (value === null) return "";
  const date = new Date(value);
  return new Date(value - date.getTimezoneOffset() * 60000)
    .toISOString()
    .slice(0, 16);
}

function TaskStatus({ status }: { status: string }) {
  const { t } = useTranslation("pages");
  return (
    <Badge variant={status === "CLOSED" ? "success" : status === "IN_PROGRESS" ? "info" : "outline"}>
      {t(`assistant.taskStatuses.${status}`, { defaultValue: status })}
    </Badge>
  );
}

function TaskEditor({
  task,
  busy,
  error,
  onCancel,
  onSave,
}: {
  task: AssistantTaskDetail | "new";
  busy: boolean;
  error: string | null;
  onCancel: () => void;
  onSave: (payload: { description: string; dueAt: number | null }) => void;
}) {
  const { t } = useTranslation("pages");
  const [description, setDescription] = useState(task === "new" ? "" : task.description ?? task.title);
  const [due, setDue] = useState(task === "new" ? "" : localDatetime(task.dueAt));
  return (
    <form
      className="space-y-4"
      onSubmit={(event) => {
        event.preventDefault();
        const payload = {
          description: description.trim(),
          dueAt: due ? new Date(due).getTime() : null,
        };
        if (payload.description && Number.isFinite(payload.dueAt ?? 0)) onSave(payload);
      }}
    >
      <Field required label={t("assistant.taskDescription")}>
        <TextArea
          required
          aria-label={t("assistant.taskDescription")}
          rows={4}
          disabled={busy}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />
      </Field>
      <Field label={t("assistant.deadlineOptional")} hint={t("assistant.deadlineHelp")}>
        <TextInput
          type="datetime-local"
          aria-label={t("assistant.deadlineOptional")}
          disabled={busy}
          value={due}
          onChange={(event) => setDue(event.target.value)}
        />
      </Field>
      {error ? <p role="alert" className="text-[var(--color-error)]">{error}</p> : null}
      <DialogFooter>
        <Button type="button" variant="outline" disabled={busy} onClick={onCancel}>
          {t("assistant.cancel")}
        </Button>
        <Button type="submit" disabled={busy || !description.trim()}>
          {t(busy ? "assistant.saving" : "assistant.save")}
        </Button>
      </DialogFooter>
    </form>
  );
}

export function AssistantTasks({
  assistantCode,
  active,
  onOpenSession,
  highlightedTask,
}: {
  assistantCode: string;
  active: boolean;
  onOpenSession: (sessionCode: string) => void;
  highlightedTask?: string | null;
}) {
  const { t, i18n } = useTranslation("pages");
  const client = useQueryClient();
  const query = useQuery({
    queryKey: assistantQueryKeys.tasks(assistantCode),
    queryFn: ({ signal }) => listAssistantTasks(assistantCode, signal),
    enabled: active,
    refetchInterval: active ? 15000 : false,
  });
  const [selectedTask, setSelectedTask] = useState<AssistantTask | null>(null);
  const [editor, setEditor] = useState<AssistantTask | "new" | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const detailCode = editor && editor !== "new" ? editor.taskCode : selectedTask?.taskCode ?? null;
  const detail = useQuery({
    queryKey: assistantQueryKeys.task(detailCode ?? ""),
    queryFn: ({ signal }) => getAssistantTask(detailCode!, signal),
    enabled: Boolean(detailCode),
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });
  const formatDate = (value: string | number) =>
    new Intl.DateTimeFormat(i18n.resolvedLanguage ?? i18n.language, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(value));

  function openEditor(task: AssistantTask | "new") {
    setSelectedTask(null);
    setEditor(task);
    setError(null);
  }

  async function mutate(action: () => Promise<unknown>) {
    setBusy(true);
    setError(null);
    try {
      await action();
      await client.invalidateQueries({ queryKey: assistantQueryKeys.tasks(assistantCode) });
      if (detailCode) void client.invalidateQueries({ queryKey: assistantQueryKeys.task(detailCode), refetchType: "none" });
      setEditor(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button size="sm" onClick={() => openEditor("new")}>
          <Plus aria-hidden="true" />{t("assistant.newTask")}
        </Button>
      </div>
      {query.isPending ? <p role="status">{t("assistant.loading")}</p> : null}
      {query.error ? (
        <div role="alert" className="flex flex-wrap items-center gap-2">
          {query.error.message}
          <Button variant="outline" size="sm" onClick={() => void query.refetch()}>{t("assistant.retry")}</Button>
        </div>
      ) : null}
      {!editor && error ? <p role="alert" className="text-[var(--color-error)]">{error}</p> : null}
      <div className="divide-y divide-[var(--color-outline-variant)]">
        {query.data?.map((task) => (
          <article
            key={task.taskCode}
            data-task-code={task.taskCode}
            className={`relative flex flex-wrap items-center gap-x-4 gap-y-2 rounded-sm px-3 py-4 transition-colors hover:bg-[var(--color-surface-container-low)] ${
              highlightedTask === task.taskCode ? "bg-[var(--color-surface-container)]" : ""
            }`}
          >
            <button
              type="button"
              aria-label={t("assistant.openTaskDetails", { title: task.title })}
              className="absolute inset-0 cursor-pointer rounded-sm focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-[var(--color-focus-ring)]"
              onClick={() => { setError(null); setSelectedTask(task); }}
            />
            <div className="pointer-events-none relative min-w-0 flex-1 basis-48">
              <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                <h3 className="min-w-0 font-medium leading-5 [overflow-wrap:anywhere]">{task.title}</h3>
                <TaskStatus status={task.status} />
              </div>
              {task.dueAt !== null || task.plannedStartAt !== null || task.nextFollowupAt !== null ? (
                <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-[var(--color-on-surface-variant)]">
                  {task.plannedStartAt !== null ? <span className="inline-flex items-center gap-1"><CalendarClock className="size-3.5 shrink-0" aria-hidden="true" />{t("assistant.plannedStart")} · {formatDate(task.plannedStartAt)}</span> : null}
                  {task.dueAt !== null ? <span>{t("assistant.deadline")} · {formatDate(task.dueAt)}</span> : null}
                  {task.nextFollowupAt !== null ? <span>{t("assistant.nextFollowup")} · {formatDate(task.nextFollowupAt)}</span> : null}
                </div>
              ) : null}
            </div>
            <div className="relative ml-auto flex shrink-0 items-center gap-1 text-[var(--color-on-surface-variant)]">
              {task.workSessionCode ? (
                <Tooltip>
                  <TooltipTrigger render={<Button type="button" size="icon" variant="ghost" aria-label={`${t("assistant.viewWork")} · ${task.title}`} onClick={() => onOpenSession(task.workSessionCode!)} />}>
                    <FileClock aria-hidden="true" />
                  </TooltipTrigger>
                  <TooltipContent>{t("assistant.viewWork")}</TooltipContent>
                </Tooltip>
              ) : null}
              <Tooltip>
                <TooltipTrigger render={<Button type="button" size="icon" variant="ghost" aria-label={`${t("assistant.editTask")} · ${task.title}`} disabled={busy} onClick={() => openEditor(task)} />}>
                  <Pencil aria-hidden="true" />
                </TooltipTrigger>
                <TooltipContent>{t("assistant.edit")}</TooltipContent>
              </Tooltip>
              {task.status !== "CLOSED" ? (
                <Tooltip>
                  <TooltipTrigger render={<Button type="button" size="icon" variant="ghost" className="hover:text-[var(--color-error)]" aria-label={`${t("assistant.cancelTask")} · ${task.title}`} disabled={busy} onClick={() => void mutate(() => cancelAssistantTask(assistantCode, task.taskCode))} />}>
                    <CircleX aria-hidden="true" />
                  </TooltipTrigger>
                  <TooltipContent>{t("assistant.cancelTask")}</TooltipContent>
                </Tooltip>
              ) : null}
            </div>
          </article>
        ))}
      </div>
      {query.data?.length === 0 ? <p className="px-3 py-4 text-sm text-[var(--color-on-surface-variant)]">{t("assistant.noTasks")}</p> : null}

      {selectedTask ? (
        <Dialog open onOpenChange={(open) => { if (!open) setSelectedTask(null); }}>
          <DialogContent closeLabel={t("assistant.close")} className="flex max-h-[85dvh] flex-col sm:max-w-2xl">
            <DialogHeader className="shrink-0 pr-8">
              <p className="text-xs text-[var(--color-on-surface-variant)]">{t("assistant.taskDetails")} · {selectedTask.taskCode}</p>
              <DialogTitle className="break-words leading-6 [overflow-wrap:anywhere]">{selectedTask.title}</DialogTitle>
            </DialogHeader>
            <div className="min-h-0 overflow-y-auto">
              {detail.isFetching ? <p role="status">{t("assistant.loading")}</p> : detail.error ? (
                <div role="alert" className="flex flex-wrap items-center gap-2 text-[var(--color-error)]">
                  {detail.error.message}
                  <Button variant="outline" size="sm" onClick={() => void detail.refetch()}>{t("assistant.retry")}</Button>
                </div>
              ) : detail.data ? (
                <div className="space-y-5">
                  <div className="flex flex-wrap items-center gap-3">
                    <TaskStatus status={detail.data.status} />
                    <span className="text-xs text-[var(--color-on-surface-variant)]">
                      {t("assistant.taskProgress")} · {new Intl.NumberFormat(i18n.resolvedLanguage ?? i18n.language, { style: "percent", maximumFractionDigits: 0 }).format(detail.data.progress / 100)}
                    </span>
                  </div>
                  <section>
                    <h3 className="text-xs font-medium text-[var(--color-on-surface-variant)]">{t("assistant.taskContent")}</h3>
                    <MarkdownContent content={detail.data.description} />
                  </section>
                  {detail.data.executionResult?.trim() ? (
                    <section>
                      <h3 className="text-xs font-medium text-[var(--color-on-surface-variant)]">{t("assistant.taskResult")}</h3>
                      <MarkdownContent content={detail.data.executionResult} />
                    </section>
                  ) : null}
                  <dl className="grid gap-x-6 gap-y-3 border-t border-[var(--color-outline-variant)] pt-4 text-sm sm:grid-cols-2">
                    {detail.data.plannedStartAt !== null ? <div><dt className="text-xs text-[var(--color-on-surface-variant)]">{t("assistant.plannedStart")}</dt><dd className="mt-1">{formatDate(detail.data.plannedStartAt)}</dd></div> : null}
                    {detail.data.dueAt !== null ? <div><dt className="text-xs text-[var(--color-on-surface-variant)]">{t("assistant.deadline")}</dt><dd className="mt-1">{formatDate(detail.data.dueAt)}</dd></div> : null}
                    {detail.data.nextFollowupAt !== null ? <div><dt className="text-xs text-[var(--color-on-surface-variant)]">{t("assistant.nextFollowup")}</dt><dd className="mt-1">{formatDate(detail.data.nextFollowupAt)}</dd></div> : null}
                    <div><dt className="text-xs text-[var(--color-on-surface-variant)]">{t("assistant.taskUpdated")}</dt><dd className="mt-1">{formatDate(detail.data.updatedAt)}</dd></div>
                  </dl>
                </div>
              ) : null}
            </div>
            <DialogFooter className="shrink-0">
              <Button variant="outline" onClick={() => setSelectedTask(null)}>{t("assistant.close")}</Button>
              {detail.data?.workSessionCode ? <Button variant="outline" onClick={() => { const sessionCode = detail.data?.workSessionCode; setSelectedTask(null); if (sessionCode) onOpenSession(sessionCode); }}>{t("assistant.viewWork")}</Button> : null}
              {detail.data ? <Button onClick={() => openEditor(selectedTask)}>{t("assistant.edit")}</Button> : null}
            </DialogFooter>
          </DialogContent>
        </Dialog>
      ) : null}

      {editor ? (
        <Dialog open disablePointerDismissal onOpenChange={(open, details) => {
          if (busy) details.cancel();
          else if (!open) setEditor(null);
        }}>
          <DialogContent closeLabel={t("assistant.close")}>
            <DialogHeader><DialogTitle>{t(editor === "new" ? "assistant.newTask" : "assistant.editTask")}</DialogTitle></DialogHeader>
            {editor === "new" || (!detail.isFetching && !detail.error && detail.data) ? (
              <TaskEditor
                task={editor === "new" ? "new" : detail.data!}
                busy={busy}
                error={error}
                onCancel={() => setEditor(null)}
                onSave={(payload) => void mutate(() => editor === "new"
                  ? createAssistantTask(assistantCode, payload)
                  : updateAssistantTask(assistantCode, editor.taskCode, payload))}
              />
            ) : detail.error ? (
              <div role="alert" className="flex flex-wrap items-center gap-2 text-[var(--color-error)]">
                {detail.error.message}
                <Button variant="outline" size="sm" onClick={() => void detail.refetch()}>{t("assistant.retry")}</Button>
              </div>
            ) : <p role="status">{t("assistant.loading")}</p>}
          </DialogContent>
        </Dialog>
      ) : null}
    </div>
  );
}
