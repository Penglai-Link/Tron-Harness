import { Check, ChevronDown, Folder, FolderX } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import { setChatWorkingDirectory } from "@/domain/chat/api/sessionLocationApi";
import type { ChatSession } from "@/domain/chat/model";
import { useDirectories } from "@/domain/directory/hooks/useDirectories";
import { getTopicMemo } from "@/domain/project/api/topicApi";

export function ChatWorkingDirectoryControl({ session, workspaceCode, draftProjectCode, draftDirectoryCode, draftSelection, onDraftSelection, disabled }: {
  session?: ChatSession | null;
  workspaceCode: string;
  draftProjectCode?: string;
  draftDirectoryCode?: string;
  draftSelection: string | null | undefined;
  onDraftSelection: (value: string | null) => void;
  disabled: boolean;
}) {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const { items, loading, error, refresh } = useDirectories(workspaceCode);
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState(false);
  const [topicDefault, setTopicDefault] = useState<string | null>(null);

  useEffect(() => {
    if (session || !draftProjectCode) { setTopicDefault(null); return; }
    const controller = new AbortController();
    getTopicMemo(draftProjectCode, controller.signal).then((memo) => {
      if (!controller.signal.aborted) setTopicDefault(memo.references.find((item) => item.kind === "Directory")?.targetCode ?? null);
    }).catch(() => { if (!controller.signal.aborted) setTopicDefault(null); });
    return () => controller.abort();
  }, [session, draftProjectCode, open]);

  const fixedDirectoryCode = session ? session.directoryCode : draftDirectoryCode;
  const currentCode = fixedDirectoryCode ?? (session
    ? session.workingDirectoryCode ?? session.directoryCode ?? null
    : draftSelection !== undefined ? draftSelection : topicDefault);
  const current = items.find((item) => item.directoryCode === currentCode);

  async function select(code: string | null) {
    if (disabled || pending || fixedDirectoryCode) return;
    if (code === currentCode) { setOpen(false); return; }
    if (!session) { onDraftSelection(code); setOpen(false); return; }
    setPending(true);
    try {
      const updated = await setChatWorkingDirectory(session.sessionCode, workspaceCode, code);
      window.dispatchEvent(new CustomEvent("tron:chat-session-updated", { detail: { session: updated } }));
      window.dispatchEvent(new CustomEvent("tron:topic-location-changed", { detail: { oldProjectCode: session.projectCode, projectCode: updated.projectCode } }));
      window.dispatchEvent(new Event("tron:project-created"));
      setOpen(false);
    } catch (reason) { toast.error(reason instanceof Error ? reason.message : t("chatSidebar.session.locationFailed")); }
    finally { setPending(false); }
  }

  if (fixedDirectoryCode) {
    return <Tooltip>
      <TooltipTrigger render={<span tabIndex={0} className="inline-flex h-8 min-w-0 items-center gap-2 rounded-md px-2 text-xs text-muted-foreground" aria-label={t("chatSidebar.session.workingDirectory")} />}>
        <Folder size={14} className="shrink-0" aria-hidden="true" /><span className="max-w-48 truncate">{current?.name ?? t("chatSidebar.directory.unavailable")}</span>
      </TooltipTrigger>
      <TooltipContent className="max-w-sm break-all">{current?.path ?? t("chatSidebar.directory.unavailable")}</TooltipContent>
    </Tooltip>;
  }

  return <>
    <Button type="button" size="sm" variant="ghost" className="h-8 min-w-0 gap-2 rounded-md px-2 text-xs text-muted-foreground hover:text-foreground" disabled={disabled || !workspaceCode} title={current?.path ?? undefined}
      onClick={() => { refresh(); setOpen(true); }} aria-label={t("chatSidebar.session.workingDirectory")}>
      <Folder size={14} className="shrink-0" aria-hidden="true" /><span className="max-w-48 truncate">{current?.name ?? (currentCode ? t("chatSidebar.directory.unavailable") : t("chatSidebar.session.noDirectory"))}</span><ChevronDown size={13} className="shrink-0" aria-hidden="true" />
    </Button>
    <Dialog open={open} onOpenChange={(value) => { if (!pending) setOpen(value); }}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader><DialogTitle>{t("chatSidebar.session.workingDirectory")}</DialogTitle></DialogHeader>
        {loading ? <p className="text-sm text-muted-foreground">{t("chatSidebar.loading")}</p> : null}
        {error ? <button type="button" className="text-left text-sm text-destructive" onClick={refresh}>{t("chatSidebar.directory.loadFailed")}</button> : null}
        <div className="max-h-80 space-y-1 overflow-y-auto rounded-lg border border-border p-1">
          {!(!session && draftProjectCode) ? <Button type="button" variant="ghost"
            aria-pressed={!currentCode}
            className={cn("h-11 w-full justify-start gap-3 rounded-md px-3 text-left", !currentCode && "bg-accent text-accent-foreground")}
            disabled={pending} onClick={() => void select(null)}>
            <FolderX size={18} className="shrink-0 text-muted-foreground" aria-hidden="true" />
            <span className="min-w-0 flex-1 truncate">{t("chatSidebar.session.noDirectory")}</span>
            {!currentCode && <Check size={16} className="shrink-0" aria-hidden="true" />}
          </Button> : null}
          {items.map((directory) => <Button type="button" key={directory.directoryCode} variant="ghost"
            aria-pressed={currentCode === directory.directoryCode} title={directory.path}
            className={cn("h-auto min-h-14 w-full justify-start gap-3 rounded-md px-3 py-2.5 text-left", currentCode === directory.directoryCode && "bg-accent text-accent-foreground")}
            disabled={pending || !directory.available} onClick={() => void select(directory.directoryCode)}>
            <Folder size={18} className="shrink-0 text-muted-foreground" aria-hidden="true" />
            <span className="min-w-0 flex-1"><span className="block truncate font-medium">{directory.name}</span><span className="block truncate text-xs font-normal text-muted-foreground">{directory.path}</span>{!directory.available && <span className="block text-xs text-destructive">{t("chatSidebar.directory.unavailable")}</span>}</span>
            {currentCode === directory.directoryCode && <Check size={16} className="shrink-0" aria-hidden="true" />}
          </Button>)}
        </div>
        <DialogFooter><Button variant="outline" disabled={pending} onClick={() => setOpen(false)}>{t("chatSidebar.project.cancel")}</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  </>;
}
