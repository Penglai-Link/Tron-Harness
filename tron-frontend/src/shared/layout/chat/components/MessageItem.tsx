import { Archive, Copy, LoaderCircle, MoreHorizontal, Pencil, Pin, PinOff, Trash2 } from "lucide-react";
import { memo, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { archiveChatSession, deleteChatSession, updateChatSession } from "@/domain/chat/hooks";
import type { ChatSession } from "@/domain/chat/model";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { useToast } from "@/components/ui/toast";
import { cn } from "@/shared/lib/utils";
import { useTabStore } from "@/store/tabStore";
import ActionButton from "./ActionButton";
import Item from "./Item";

const MessageItem = ({
  active, onRemovePin, onTogglePin, pinned, running, session,
}: {
  active: boolean;
  onNavigate: (path: string) => void;
  onRemovePin: () => void;
  onTogglePin: () => void;
  pinned: boolean;
  running: boolean;
  session: ChatSession;
}) => {
  const toast = useToast();
  const { t } = useTranslation("shell");
  const [pendingAction, setPendingAction] = useState<"archive" | "delete" | null>(null);
  const [renameOpen, setRenameOpen] = useState(false);
  const [renameTitle, setRenameTitle] = useState(session.title);
  const [renaming, setRenaming] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const sessionParams = new URLSearchParams({ sessionCode: session.sessionCode });
  if (session.workspaceCode) sessionParams.set("workspaceCode", session.workspaceCode);
  const sessionPath = `/chats?${sessionParams}`;

  async function archive() {
    if (pendingAction || running) return;
    setPendingAction("archive");
    try {
      await archiveChatSession(session.sessionCode);
      toast.success(t("shell:chatSidebar.session.archived"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("shell:chatSidebar.session.archiveFailed"));
    } finally {
      setPendingAction(null);
    }
  }

  async function remove() {
    if (pendingAction || running) return;
    setPendingAction("delete");
    try {
      await deleteChatSession(session.sessionCode);
      onRemovePin();
      setDeleteOpen(false);
      toast.success(t("shell:chatSidebar.session.deleted"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("shell:chatSidebar.session.deleteFailed"));
    } finally {
      setPendingAction(null);
    }
  }

  async function rename() {
    const title = renameTitle.trim();
    if (!title || renaming || title === session.title) {
      if (title === session.title) setRenameOpen(false);
      return;
    }
    setRenaming(true);
    try {
      const updated = await updateChatSession(session.sessionCode, { title });
      window.dispatchEvent(new CustomEvent("tron:chat-session-updated", { detail: { session: updated } }));
      const tabStore = useTabStore.getState();
      const sessionTab = tabStore.getAllTabs().find((tab) => tab.activeNav === `chat-session:${session.sessionCode}`);
      if (sessionTab) tabStore.updateTab(sessionTab.id, { title: updated.title });
      setRenameOpen(false);
      toast.success(t("shell:chatSidebar.session.renamed"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("shell:chatSidebar.session.renameFailed"));
    } finally {
      setRenaming(false);
    }
  }

  async function copySessionId() {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(session.sessionCode);
      } else {
        const textarea = document.createElement("textarea");
        textarea.value = session.sessionCode;
        textarea.style.position = "fixed";
        textarea.style.left = "-9999px";
        document.body.appendChild(textarea);
        textarea.select();
        const copied = document.execCommand("copy");
        textarea.remove();
        if (!copied) throw new Error("Clipboard copy failed");
      }
      toast.success(t("shell:chatSidebar.session.idCopied"));
    } catch {
      toast.error(t("shell:chatSidebar.session.copyFailed"));
    }
  }

  return (
    <>
      <Item
        className={cn("group/session relative flex h-8 min-w-0 items-center pl-6", active && "bg-sidebar-accent text-sidebar-accent-foreground")}
        left={
          <Link
            to={sessionPath}
            className="flex min-w-0 flex-1 items-center gap-1.5 truncate text-sm"
            onClick={(event) => {
              if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
              if (session.workspaceCode && useTabStore.getState().workspaceCode !== session.workspaceCode) return;
              const path = sessionPath;
              useTabStore.getState().addTab({ title: session.title, path, closable: true, activeNav: `chat-session:${session.sessionCode}` });
            }}
          >
            {pinned ? <Pin size={12} className="shrink-0 opacity-70" aria-label={t("shell:chatSidebar.session.pinned")} /> : null}
            <span className="min-w-0 truncate">{session.title}</span>
          </Link>
        }
        right={
          <div className={cn(
            "pointer-events-none flex h-full shrink-0 items-center gap-1 pl-2 opacity-0 transition-opacity",
            "group-hover/session:pointer-events-auto group-hover/session:opacity-100",
            "group-focus-within/session:pointer-events-auto group-focus-within/session:opacity-100",
            "has-[button[data-popup-open]]:pointer-events-auto has-[button[data-popup-open]]:opacity-100",
            running && "pointer-events-auto opacity-100",
          )}>
            <DropdownMenu>
              <DropdownMenuTrigger render={
                <ActionButton needHover className="size-6 justify-center rounded hover:bg-sidebar-accent" onClick={(event) => event.stopPropagation()} aria-label={t("common:actions.more", "More")}>
                  <MoreHorizontal size={16} aria-hidden="true" />
                </ActionButton>
              } />
              <DropdownMenuContent className="min-w-44 duration-10" align="end" onClick={(event) => event.stopPropagation()}>
                <DropdownMenuItem onClick={onTogglePin}>
                  {pinned ? <PinOff aria-hidden="true" /> : <Pin aria-hidden="true" />}
                  {t(pinned ? "shell:chatSidebar.session.unpin" : "shell:chatSidebar.session.pin")}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => { setRenameTitle(session.title); setRenameOpen(true); }}>
                  <Pencil aria-hidden="true" />{t("shell:chatSidebar.session.rename")}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => void copySessionId()}>
                  <Copy aria-hidden="true" />{t("shell:chatSidebar.session.copyId")}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem variant="destructive" disabled={running || Boolean(pendingAction)} onClick={() => setDeleteOpen(true)}>
                  <Trash2 aria-hidden="true" />{t("shell:chatSidebar.session.delete")}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <ActionButton needHover className="size-6 shrink-0 justify-center rounded hover:bg-sidebar-accent" onClick={(event) => { event.preventDefault(); event.stopPropagation(); void archive(); }} disabled={Boolean(pendingAction) || running} aria-label={t("common:actions.archive", "Archive")}>
              {pendingAction === "archive" ? <LoaderCircle size={14} className="animate-spin" aria-hidden="true" /> : <Archive size={16} aria-hidden="true" />}
            </ActionButton>
            {running ? <LoaderCircle aria-label={t("common:status.running", "Running")} size={14} className="animate-spin" /> : null}
          </div>
        }
      />
      <Dialog open={renameOpen} onOpenChange={setRenameOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader><DialogTitle>{t("shell:chatSidebar.session.rename")}</DialogTitle></DialogHeader>
          <form onSubmit={(event) => { event.preventDefault(); void rename(); }}>
            <Input autoFocus aria-label={t("shell:chatSidebar.session.title")} value={renameTitle} maxLength={200} onChange={(event) => setRenameTitle(event.target.value)} />
            <DialogFooter className="mt-5">
              <Button type="button" variant="ghost" onClick={() => setRenameOpen(false)} disabled={renaming}>{t("shell:chatSidebar.project.cancel")}</Button>
              <Button type="submit" disabled={!renameTitle.trim() || renaming}>{t("shell:chatSidebar.session.save")}</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={deleteOpen}
        pending={pendingAction === "delete"}
        title={t("shell:chatSidebar.session.deleteTitle")}
        description={t("shell:chatSidebar.session.deleteDescription", { name: session.title })}
        cancelText={t("shell:chatSidebar.project.cancel")}
        confirmText={t("shell:chatSidebar.session.delete")}
        onCancel={() => setDeleteOpen(false)}
        onConfirm={() => void remove()}
      />
    </>
  );
};

export default memo(MessageItem);
