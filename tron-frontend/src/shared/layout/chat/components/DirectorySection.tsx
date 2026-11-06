import { Folder, FolderOpen, MoreHorizontal, Plus } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useToast } from "@/components/ui/toast";
import type { ChatSession } from "@/domain/chat/model";
import { notifyArchivedChatSessions } from "@/domain/chat/hooks/useChatSessionGroupList";
import { archiveDirectory, renameDirectory, type DirectoryRecord } from "@/domain/directory/api/directoryApi";
import { useDirectories } from "@/domain/directory/hooks/useDirectories";
import { useCreateChatTab } from "../hooks/useCreateChatTab";
import { sortPinnedSessions } from "../hooks/usePinnedSessionCodes";
import ActionButton from "./ActionButton";
import Item from "./Item";
import MessageItem from "./MessageItem";
import { SectionHeader } from "./SectionHeader";

interface DirectorySectionProps {
  label: string;
  onAdd: () => void;
  adding: boolean;
  workspaceCode: string;
  sessions: Map<string, ChatSession[]>;
  runningSessionCode: string;
  activeSessionCode: string;
  pinnedSessionCodes: string[];
  onNavigate: (path: string) => void;
  onToggleSessionPin: (code: string) => void;
  onRemoveSessionPin: (code: string) => void;
}

type TreeNode = { directory: DirectoryRecord; children: TreeNode[] };

function makeTree(items: DirectoryRecord[]): TreeNode[] {
  const nodes = new Map(items.map((directory) => [directory.directoryCode, { directory, children: [] as TreeNode[] }]));
  const roots: TreeNode[] = [];
  items.forEach((directory) => {
    const node = nodes.get(directory.directoryCode)!;
    const parent = directory.parentDirectoryCode ? nodes.get(directory.parentDirectoryCode) : undefined;
    if (parent) parent.children.push(node);
    else roots.push(node);
  });
  return roots;
}

export function DirectorySection(props: DirectorySectionProps) {
  const { t } = useTranslation("shell");
  const { items, loading, error, refresh } = useDirectories(props.workspaceCode);
  const tree = useMemo(() => makeTree(items), [items]);
  const [open, setOpen] = useState(true);

  return (
    <section>
      <SectionHeader
        label={props.label}
        open={open}
        onToggle={() => setOpen((value) => !value)}
        onCreate={props.onAdd}
        createDisabled={!props.workspaceCode || props.adding}
        createLabel={t("chatSidebar.directory.add")}
        shortcutAction="newDirectory"
      />
      {open && loading && !items.length ? <p className="px-2 text-sm opacity-50">{t("chatSidebar.loading")}</p> : null}
      {open && error ? <button type="button" className="px-2 text-left text-sm text-destructive" onClick={refresh}>{t("chatSidebar.directory.loadFailed")}</button> : null}
      {open ? tree.map((node) => <DirectoryNode key={node.directory.directoryCode} node={node} {...props} refresh={refresh} />) : null}

    </section>
  );
}

function DirectoryNode({ node, refresh, ...props }: DirectorySectionProps & { node: TreeNode; refresh: () => void }) {
  const { t } = useTranslation("shell");
  const toast = useToast();
  const directory = node.directory;
  const [open, setOpen] = useState(true);
  const [renameOpen, setRenameOpen] = useState(false);
  const [archiveOpen, setArchiveOpen] = useState(false);
  const [name, setName] = useState(directory.name);
  const [pending, setPending] = useState(false);
  const { create } = useCreateChatTab(directory.workspaceCode, undefined, directory.directoryCode);
  const Icon = open ? FolderOpen : Folder;
  const sessions = props.sessions.get(directory.directoryCode) ?? [];

  async function copyPath() {
    try {
      await navigator.clipboard.writeText(directory.path);
      toast.success(t("chatSidebar.directory.pathCopied"));
    } catch { toast.error(t("chatSidebar.directory.copyFailed")); }
  }

  async function rename() {
    if (!name.trim() || pending) return;
    setPending(true);
    try { await renameDirectory(directory.directoryCode, name.trim()); setRenameOpen(false); window.dispatchEvent(new Event("tron:directories-changed")); }
    catch (reason) { toast.error(reason instanceof Error ? reason.message : t("chatSidebar.directory.saveFailed")); }
    finally { setPending(false); }
  }

  async function archive() {
    if (pending) return;
    setPending(true);
    try {
      const archived = await archiveDirectory(directory.directoryCode);
      notifyArchivedChatSessions(archived.sessionCodes);
      setArchiveOpen(false);
      window.dispatchEvent(new Event("tron:directories-changed"));
      window.dispatchEvent(new Event("tron:project-created"));
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : t("chatSidebar.directory.archiveFailed"));
      window.dispatchEvent(new Event("tron:project-created"));
    } finally { setPending(false); }
  }

  return (
    <section>
      <Item className="group/directory flex min-w-0 items-center gap-1 pl-2" left={
        <Tooltip><TooltipTrigger render={<button type="button" aria-expanded={open} className="flex min-w-0 flex-1 items-center gap-2 truncate text-left" onClick={() => setOpen((value) => !value)}>
          <Icon size={16} className="shrink-0" aria-hidden="true" /><span className="truncate">{directory.name}</span>{!directory.available ? <span className="text-xs opacity-60">{t("chatSidebar.directory.unavailable")}</span> : null}
        </button>} /><TooltipContent side="bottom" align="start" sideOffset={8} className="max-w-sm break-all">{directory.path}</TooltipContent></Tooltip>
      } right={
        <div className="flex shrink-0 items-center gap-2">
          <ActionButton aria-label={t("chatSidebar.directory.newChat", { name: directory.name })} onClick={() => create()}><Plus size={15} aria-hidden="true" /></ActionButton>
          <DropdownMenu><DropdownMenuTrigger render={<ActionButton aria-label={t("common:actions.more")}><MoreHorizontal size={16} aria-hidden="true" /></ActionButton>} />
            <DropdownMenuContent align="start"><DropdownMenuItem onClick={() => void copyPath()}>{t("chatSidebar.directory.copyPath")}</DropdownMenuItem><DropdownMenuItem onClick={() => { setName(directory.name); setRenameOpen(true); }}>{t("chatSidebar.directory.rename")}</DropdownMenuItem><DropdownMenuItem onClick={() => setArchiveOpen(true)}>{t("chatSidebar.directory.archive")}</DropdownMenuItem></DropdownMenuContent>
          </DropdownMenu>
        </div>
      } />
      {open ? <div className="pl-3">
        {node.children.map((child) => <DirectoryNode key={child.directory.directoryCode} node={child} {...props} refresh={refresh} />)}
        {sortPinnedSessions(sessions, props.pinnedSessionCodes).map((session) => <MessageItem key={session.sessionCode} session={session} running={props.runningSessionCode === session.sessionCode} active={props.activeSessionCode === session.sessionCode} onNavigate={props.onNavigate} pinned={props.pinnedSessionCodes.includes(session.sessionCode)} onTogglePin={() => props.onToggleSessionPin(session.sessionCode)} onRemovePin={() => props.onRemoveSessionPin(session.sessionCode)} />)}
      </div> : null}
      <Dialog open={renameOpen} onOpenChange={setRenameOpen}><DialogContent><DialogHeader><DialogTitle>{t("chatSidebar.directory.rename")}</DialogTitle></DialogHeader><Field><FieldLabel htmlFor={`directory-name-${directory.directoryCode}`}>{t("chatSidebar.directory.name")}</FieldLabel><Input id={`directory-name-${directory.directoryCode}`} value={name} onChange={(event) => setName(event.target.value)} /></Field><DialogFooter><Button variant="outline" onClick={() => setRenameOpen(false)}>{t("chatSidebar.directory.cancel")}</Button><Button disabled={!name.trim() || pending} onClick={() => void rename()}>{t("chatSidebar.directory.save")}</Button></DialogFooter></DialogContent></Dialog>
      <ConfirmDialog open={archiveOpen} pending={pending} title={t("chatSidebar.directory.archiveTitle")} description={t("chatSidebar.directory.archiveDescription", { name: directory.name })} cancelText={t("chatSidebar.directory.cancel")} confirmText={t("chatSidebar.directory.archive")} onCancel={() => setArchiveOpen(false)} onConfirm={() => void archive()} />
    </section>
  );
}
