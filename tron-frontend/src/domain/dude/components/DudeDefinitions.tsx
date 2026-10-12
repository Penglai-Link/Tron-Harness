import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { AgentAvatarWithActivity } from "@/domain/agent/components/AgentAvatarWithActivity";
import { useDudes } from "../hooks/useDudes";
import {
  deleteDude,
  dudeQueryKeys,
  type Dude,
} from "../api/dudeApi";
import { DudeEditorDialog } from "./DudeEditorDialog";
import { SurfaceCard } from "@/shared";
import { ShieldCheck } from "lucide-react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { InfoTooltip } from "@/components/info-tooltip";

export function DudeDefinitions({ embedded = false, title, titleHelp, titleHelpAriaLabel }: { embedded?: boolean; title?: string; titleHelp?: string; titleHelpAriaLabel?: string }) {
  const { t } = useTranslation("pages");
  const queryClient = useQueryClient();
  const query = useDudes();
  const [editor, setEditor] = useState<Dude | "new" | null>(null);
  const [deleting, setDeleting] = useState<Dude | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: dudeQueryKeys.all });
  async function change(action: () => Promise<unknown>) {
    setBusy(true);
    setError(null);
    try {
      await action();
      await refresh();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className={embedded ? "space-y-3" : "space-y-4"} aria-label={title ?? t("dude.title")}>
      {embedded ? (
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-1">
            <h2 className="text-base font-semibold">{title ?? t("dude.category")}</h2>
            {titleHelp ? <InfoTooltip ariaLabel={titleHelpAriaLabel ?? titleHelp}>{titleHelp}</InfoTooltip> : null}
          </div>
          <Button onClick={() => setEditor("new")}>{t("workspaceAgents.internal.definition.create")}</Button>
        </div>
      ) : <div className="flex items-center justify-end py-3">
        <Button onClick={() => setEditor("new")}>{t("workspaceAgents.internal.definition.create")}</Button>
      </div>}
      {query.isPending ? <p role="status">{t("dude.loading")}</p> : null}
      {query.error || error ? (
        <div
          role="alert"
          className="flex items-center gap-3 text-[var(--color-error)]"
        >
          <span>{error ?? query.error?.message}</span>
          <Button
            variant="outline"
            onClick={() => {
              setError(null);
              void query.refetch();
            }}
          >
            {t("dude.retry")}
          </Button>
        </div>
      ) : null}
      <SurfaceCard className="min-h-28 p-3">
        <div className="grid grid-cols-[repeat(auto-fill,minmax(112px,1fr))] gap-3 sm:grid-cols-[repeat(auto-fill,minmax(128px,1fr))]">
        {query.data?.map((dude) => (
          <div key={dude.dudeCode} className="relative min-w-0">
            <Button variant="ghost" className="h-auto w-full flex-col gap-2 whitespace-normal rounded-xl px-2 py-3 text-center" disabled={busy} onClick={() => setEditor(dude)}>
              <AgentAvatarWithActivity
                active={dude.enabled}
                avatarId={dude.avatarId}
                name={dude.name}
              />
              <span className="block w-full truncate text-sm font-medium">{dude.name}</span>
              <span className="block w-full truncate font-mono text-xs text-[var(--color-on-surface-variant)]">{dude.dudeCode}</span>
            </Button>
            {dude.kind === "Main" || dude.kind === "Memory" ? (
              <Tooltip>
                <TooltipTrigger
                  aria-label={t("workspaceAgents.internal.groups.builtIn", { defaultValue: "Built-in agent" })}
                  className="absolute left-[calc(50%+1.5rem)] top-[5.25rem] flex size-6 items-center justify-center rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] text-[var(--color-on-surface-variant)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)]"
                >
                  <ShieldCheck size={14} aria-hidden="true" />
                </TooltipTrigger>
                <TooltipContent>{t("workspaceAgents.internal.groups.builtIn", { defaultValue: "Built-in agent" })}</TooltipContent>
              </Tooltip>
            ) : null}
            {dude.kind === "Custom" && !dude.isMain ? (
              <Button type="button" size="icon-sm" variant="ghost" aria-label={t("dude.delete")} title={t("dude.delete")} disabled={busy} className="absolute right-1 top-1" onClick={() => setDeleting(dude)}>×</Button>
            ) : null}
          </div>
        ))}
        </div>
      </SurfaceCard>
      {query.data?.length === 0 ? <p>{t("dude.empty")}</p> : null}
      {editor ? (
        <DudeEditorDialog
          dude={editor === "new" ? undefined : editor}
          onClose={() => setEditor(null)}
          onSaved={refresh}
        />
      ) : null}
      {deleting?.kind === "Custom" && !deleting.isMain ? (
        <Dialog
          open
          onOpenChange={(open) => {
            if (!open && !busy) setDeleting(null);
          }}
        >
          <DialogContent closeLabel={t("dude.close")}>
            <DialogHeader>
              <DialogTitle>{t("dude.delete")}</DialogTitle>
              <DialogDescription>
                {t("dude.deleteConfirm", { name: deleting.name })}
              </DialogDescription>
            </DialogHeader>
            {error ? (
              <p role="alert" className="text-sm text-[var(--color-error)]">
                {error}
              </p>
            ) : null}
            <DialogFooter>
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => setDeleting(null)}
              >
                {t("dude.cancel")}
              </Button>
              <Button
                variant="destructive"
                disabled={busy}
                onClick={() =>
                  void change(async () => {
                    await deleteDude(deleting.dudeCode);
                    setDeleting(null);
                  })
                }
              >
                {t("dude.delete")}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      ) : null}
    </section>
  );
}
