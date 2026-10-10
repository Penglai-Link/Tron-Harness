import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation } from "@tanstack/react-query";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { usePlatformWorkspaces } from "./PlatformWorkspaceScope";

export function WorkspaceAssignmentDialog({ name, assigned, save, onClose, onSaved, confirmLabel, help }: {
  name: string; assigned: string[]; save: (codes: string[]) => Promise<unknown>; onClose: () => void; onSaved: () => void;
  confirmLabel?: string; help?: string;
}) {
  const { t } = useTranslation("pages");
  const query = usePlatformWorkspaces();
  const [selected, setSelected] = useState(() => new Set(assigned));
  const [search, setSearch] = useState("");
  const mutation = useMutation({ mutationFn: () => save([...selected]), onSuccess: () => { onSaved(); onClose(); } });
  return <Dialog open onOpenChange={(open) => { if (!open && !mutation.isPending) onClose(); }}>
    <DialogContent closeLabel={t("platformData.cancel")}><DialogHeader><DialogTitle>{t("platformData.assignTitle", { name })}</DialogTitle></DialogHeader>
      <TextInput type="search" aria-label={t("platformData.searchWorkspaces")} placeholder={t("platformData.searchWorkspaces")} value={search} onChange={(event) => setSearch(event.target.value)} />
      <div className="grid max-h-80 gap-3 overflow-y-auto">
        {(query.data ?? []).filter((option) => option.workspaceCode !== "PLATFORM" && `${option.name} ${option.label} ${option.workspaceCode}`.toLowerCase().includes(search.toLowerCase())).map((option) => <label key={option.workspaceCode} className="flex items-center gap-3 text-sm">
          <input type="checkbox" checked={selected.has(option.workspaceCode)} disabled={mutation.isPending} onChange={(event) => setSelected((current) => {
            const next = new Set(current); if (event.target.checked) next.add(option.workspaceCode); else next.delete(option.workspaceCode); return next;
          })} />{option.label || option.name}
        </label>)}
      </div>
      <p className="text-sm text-[var(--color-on-surface-variant)]">{help ?? t("platformData.assignmentHelp")}</p>
      {query.error || mutation.error ? <p role="alert">{(query.error ?? mutation.error)?.message}</p> : null}
      <DialogFooter><Button type="button" variant="outline" disabled={mutation.isPending} onClick={onClose}>{t("platformData.cancel")}</Button><Button type="button" disabled={query.isLoading || Boolean(query.error) || mutation.isPending} onClick={() => mutation.mutate()}>{confirmLabel ?? t("platformData.save")}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}
