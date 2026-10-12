import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { InternalAgentMcpServerSelector, InternalAgentToolSelector } from "@/domain/agent/components";
import type { PlatformToolRecord, ToolCatalogDomainRecord } from "@/domain/agent/model";
import type { WorkspaceMcpServerRecord } from "@/domain/mcp/model";

export type InternalAgentAssignmentKind = "mcp" | "tools";

interface InternalAgentAssignmentDialogProps {
  kind: InternalAgentAssignmentKind;
  agentName: string;
  busy: boolean;
  initialSelectedCodes: string[];
  servers: WorkspaceMcpServerRecord[];
  tools: PlatformToolRecord[];
  toolDomains: ToolCatalogDomainRecord[];
  onClose: () => void;
  onConfirm: (codes: string[]) => void;
}

export function InternalAgentAssignmentDialog({
  kind, agentName, busy, initialSelectedCodes, servers, tools, toolDomains, onClose, onConfirm,
}: InternalAgentAssignmentDialogProps) {
  const { t } = useTranslation(["pages"]);
  const [selectedCodes, setSelectedCodes] = useState(() => [...initialSelectedCodes]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:workspaceAgents.internal.assignmentDialog.${key}`, { defaultValue, ...options })
  );
  const setChecked = (codes: string[], checked: boolean) => {
    setSelectedCodes((current) => {
      const next = new Set(current);
      codes.forEach((code) => { if (checked) next.add(code); else next.delete(code); });
      return [...next].sort((left, right) => left.localeCompare(right));
    });
  };

  return (
    <Dialog disablePointerDismissal open onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent className="flex max-h-[90dvh] flex-col overflow-hidden sm:max-w-6xl" closeLabel={pageT("close", "Close")}>
        <DialogHeader>
          <DialogTitle>{kind === "mcp" ? pageT("mcpTitle", "Choose MCP servers") : pageT("toolsTitle", "Choose tools")}</DialogTitle>
          <DialogDescription>{agentName} · {pageT("selectedCount", "{{count}} selected", { count: selectedCodes.length })}</DialogDescription>
        </DialogHeader>
        <div className="min-h-0 overflow-y-auto">
          {kind === "mcp" ? (
            <InternalAgentMcpServerSelector className="mt-0" busy={busy} servers={servers} selectedServerCodes={selectedCodes}
              onToggleServer={(code, checked) => setChecked([code], checked)} onSetServersChecked={setChecked} />
          ) : (
            <InternalAgentToolSelector className="mt-0" busy={busy} tools={tools} toolDomains={toolDomains} selectedToolCodes={selectedCodes}
              onToggleTool={(code, checked) => setChecked([code], checked)} />
          )}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>{pageT("cancel", "Cancel")}</Button>
          <Button type="button" disabled={busy} onClick={() => onConfirm(selectedCodes)}>{pageT("confirm", "Confirm")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
