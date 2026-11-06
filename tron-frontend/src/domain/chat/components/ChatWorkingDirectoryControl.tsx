import { ChevronDown, Folder, FolderX, Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuRadioGroup,
  DropdownMenuRadioItem, DropdownMenuSeparator, DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useToast } from "@/components/ui/toast";
import { setChatWorkingDirectory } from "@/domain/chat/api/sessionLocationApi";
import type { ChatSession } from "@/domain/chat/model";
import { useDirectories } from "@/domain/directory/hooks/useDirectories";
import { useSystemDirectoryPicker } from "@/domain/directory/hooks/useSystemDirectoryPicker";
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

  const picker = useSystemDirectoryPicker(workspaceCode, (directory) => select(directory.directoryCode));
  const busy = pending || picker.saving;

  if (fixedDirectoryCode) {
    return <Tooltip>
      <TooltipTrigger render={<span tabIndex={0} className="inline-flex h-8 min-w-0 items-center gap-2 rounded-md px-2 text-xs text-muted-foreground" aria-label={t("chatSidebar.session.workingDirectory")} />}>
        <Folder size={14} className="shrink-0" aria-hidden="true" /><span className="max-w-48 truncate">{current?.name ?? t("chatSidebar.directory.unavailable")}</span>
      </TooltipTrigger>
      <TooltipContent className="max-w-sm break-all">{current?.path ?? t("chatSidebar.directory.unavailable")}</TooltipContent>
    </Tooltip>;
  }

  return <DropdownMenu open={open} onOpenChange={(value) => {
    if (value) refresh();
    setOpen(value);
  }}>
    <DropdownMenuTrigger render={<Button type="button" size="sm" variant="ghost"
      className="h-8 min-w-0 gap-2 rounded-md px-2 text-xs text-muted-foreground hover:text-foreground data-popup-open:bg-accent data-popup-open:text-foreground"
      disabled={disabled || busy || !workspaceCode} title={current?.path ?? undefined}
      aria-label={t("chatSidebar.session.workingDirectory")} />}>
      <Folder size={14} className="shrink-0" aria-hidden="true" />
      <span className="max-w-48 truncate">{current?.name ?? (currentCode ? t("chatSidebar.directory.unavailable") : t("chatSidebar.session.noDirectory"))}</span>
      <ChevronDown size={13} className="shrink-0" aria-hidden="true" />
    </DropdownMenuTrigger>
    <DropdownMenuContent align="start" side="top" sideOffset={6}
      className="w-64 max-w-[calc(100vw-2rem)] p-1.5" aria-label={t("chatSidebar.session.workingDirectory")}>
      {loading && !items.length ? <div role="status" className="px-2 py-2 text-xs text-muted-foreground">{t("chatSidebar.loading")}</div> : null}
      {error ? <DropdownMenuItem closeOnClick={false} className="min-h-9 px-2 text-destructive" onClick={refresh}>{t("chatSidebar.directory.loadFailed")}</DropdownMenuItem> : null}
      <DropdownMenuRadioGroup value={currentCode ?? ""} onValueChange={(value) => void select(value || null)} className="max-h-72 overflow-y-auto">
        {items.map((directory) => <Tooltip key={directory.directoryCode}>
          <TooltipTrigger render={<DropdownMenuRadioItem value={directory.directoryCode} label={directory.name}
            closeOnClick disabled={busy || !directory.available} className="min-h-9 gap-2 px-2 pr-8 text-xs" />}>
            <Folder size={14} className="text-muted-foreground" aria-hidden="true" />
            <span className="min-w-0 flex-1 truncate">{directory.name}</span>
            {!directory.available && <span className="text-xs text-muted-foreground">{t("chatSidebar.directory.unavailable")}</span>}
          </TooltipTrigger>
          <TooltipContent side="right" sideOffset={10} className="max-w-sm break-all">{directory.path}</TooltipContent>
        </Tooltip>)}
        {!(!session && draftProjectCode) ? <DropdownMenuRadioItem value="" closeOnClick disabled={busy} className="min-h-9 gap-2 px-2 pr-8 text-xs">
          <FolderX size={14} className="text-muted-foreground" aria-hidden="true" />
          <span className="truncate">{t("chatSidebar.session.noDirectory")}</span>
        </DropdownMenuRadioItem> : null}
      </DropdownMenuRadioGroup>
      <DropdownMenuSeparator className="mx-0 my-1" />
      <DropdownMenuItem disabled={busy || disabled} className="min-h-9 gap-2 px-2 text-xs" onClick={() => { setOpen(false); void picker.choose(); }}>
        <Plus size={14} className="text-muted-foreground" aria-hidden="true" />
        {t("chatSidebar.directory.add")}…
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>;
}
