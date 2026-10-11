import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { AgentAvatar } from "@/domain/agent/components/AgentAvatarPicker";
import { useDudes } from "../hooks/useDudes";
import { deleteDude, dudeQueryKeys, setMainDude, updateDude, type Dude } from "../api/dudeApi";
import { DudeEditorDialog } from "./DudeEditorDialog";

export function DudeDefinitions() {
  const { t } = useTranslation("pages"); const queryClient = useQueryClient();
  const query = useDudes(); const [editor, setEditor] = useState<Dude | "new" | null>(null);
  const [deleting, setDeleting] = useState<Dude | null>(null); const [busy, setBusy] = useState(false); const [error, setError] = useState<string | null>(null);
  const refresh = () => queryClient.invalidateQueries({ queryKey: dudeQueryKeys.all });
  async function change(action: () => Promise<unknown>) { setBusy(true); setError(null); try { await action(); await refresh(); } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); } finally { setBusy(false); } }
  return <section className="space-y-4" aria-label={t("dude.title")}>
    <div className="flex items-center justify-end"><Button onClick={() => setEditor("new")}>{t("dude.create")}</Button></div>
    {query.isPending ? <p role="status">{t("dude.loading")}</p> : null}
    {query.error || error ? <div role="alert" className="flex items-center gap-3 text-[var(--color-error)]"><span>{error ?? query.error?.message}</span><Button variant="outline" onClick={() => { setError(null); void query.refetch(); }}>{t("dude.retry")}</Button></div> : null}
    <div className="divide-y divide-[var(--color-outline-variant)]">
      {query.data?.map(dude => <div key={dude.dudeCode} className="flex flex-wrap items-center gap-3 py-3">
        <AgentAvatar avatarId={dude.avatarId} className="size-10 rounded-md" />
        <div className="min-w-0 flex-1"><div className="flex items-center gap-2"><span className="font-medium">{dude.name}</span>{dude.isMain ? <Badge variant="secondary">{t("dude.main")}</Badge> : null}{dude.kind === "Memory" ? <Badge variant="outline">{t("dude.memory")}</Badge> : null}</div><p className="text-xs text-[var(--color-on-surface-variant)]">{dude.dudeCode} · {t(dude.enabled ? "dude.running" : "dude.paused")}</p></div>
        <Button size="sm" variant="ghost" nativeButton={false} render={<Link to={`/dudes?dude=${encodeURIComponent(dude.dudeCode)}`} />}>{t("dude.open")}</Button>
        <Button size="sm" variant="outline" disabled={busy} onClick={() => setEditor(dude)}>{t("dude.edit")}</Button>
        <Button size="sm" variant="outline" disabled={busy} onClick={() => void change(() => updateDude(dude.dudeCode, { enabled: !dude.enabled }))}>{t(dude.enabled ? "dude.pause" : "dude.resume")}</Button>
        {!dude.isMain ? <Button size="sm" variant="outline" disabled={busy} onClick={() => void change(() => setMainDude(dude.dudeCode))}>{t("dude.makeMain")}</Button> : null}
        {!dude.isMain ? <Button size="sm" variant="ghost" disabled={busy} onClick={() => setDeleting(dude)}>{t("dude.delete")}</Button> : null}
      </div>)}
    </div>
    {query.data?.length === 0 ? <p>{t("dude.empty")}</p> : null}
    {editor ? <DudeEditorDialog dude={editor === "new" ? undefined : editor} onClose={() => setEditor(null)} onSaved={refresh} /> : null}
    {deleting ? <Dialog open onOpenChange={open => { if (!open && !busy) setDeleting(null); }}><DialogContent closeLabel={t("dude.close")}><DialogHeader><DialogTitle>{t("dude.delete")}</DialogTitle><DialogDescription>{t("dude.deleteConfirm", { name: deleting.name })}</DialogDescription></DialogHeader>{error ? <p role="alert" className="text-sm text-[var(--color-error)]">{error}</p> : null}<DialogFooter><Button variant="outline" disabled={busy} onClick={() => setDeleting(null)}>{t("dude.cancel")}</Button><Button variant="destructive" disabled={busy} onClick={() => void change(async () => { await deleteDude(deleting.dudeCode); setDeleting(null); })}>{t("dude.delete")}</Button></DialogFooter></DialogContent></Dialog> : null}
  </section>;
}
