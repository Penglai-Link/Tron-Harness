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
  allowRemovingUnavailable?: boolean;
  switchingFromRule?: boolean;
}

export function InternalAgentAssignmentDialog({
  kind, agentName, busy, initialSelectedCodes, servers, tools, toolDomains, onClose, onConfirm, allowRemovingUnavailable = false, switchingFromRule = false,
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
      <DialogContent className="flex h-[min(48rem,90dvh)] w-[calc(100vw-2rem)] flex-col overflow-hidden sm:max-w-6xl" closeLabel={pageT("close", "Close")}>
        <DialogHeader className="shrink-0">
          <DialogTitle>{kind === "mcp" ? pageT("mcpTitle", "Choose MCP servers") : pageT("toolsTitle", "Choose tools")}</DialogTitle>
          <DialogDescription>{agentName} · {pageT("selectedCount", "{{count}} selected", { count: selectedCodes.length })}</DialogDescription>
          {switchingFromRule && <p className="text-xs text-[var(--color-on-surface-variant)]">{t("pages:workspaceAgents.internal.dudeToolPolicy.switchToFixedHint")}</p>}
        </DialogHeader>
        <div className={kind === "tools" ? "min-h-0 flex-1 overflow-hidden" : "tron-scrollbar min-h-0 flex-1 overflow-x-hidden overflow-y-auto"}>
          {kind === "mcp" ? (
            <InternalAgentMcpServerSelector className="mt-0" busy={busy} servers={servers} selectedServerCodes={selectedCodes}
              onToggleServer={(code, checked) => setChecked([code], checked)} onSetServersChecked={setChecked} />
          ) : (
            <InternalAgentToolSelector className="mt-0" busy={busy} tools={tools} toolDomains={toolDomains} selectedToolCodes={selectedCodes}
              allowRemovingUnavailable={allowRemovingUnavailable}
              onSetToolsChecked={allowRemovingUnavailable ? setChecked : undefined}
              onToggleTool={(code, checked) => setChecked([code], checked)} />
          )}
        </div>
        <DialogFooter className="shrink-0">
          <Button type="button" variant="outline" onClick={onClose}>{pageT("cancel", "Cancel")}</Button>
          <Button type="button" disabled={busy} onClick={() => onConfirm(selectedCodes)}>{pageT("confirm", "Confirm")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
