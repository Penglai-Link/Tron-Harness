import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Maximize2, Minimize2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogClose, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { ResourceFilePreview } from "./ResourceFilePreview";
import type { ResourceKind } from "../model/ResourceModel";

export function ResourceFilePreviewDialog({ workspaceCode, resourceCode, fileName, kind, revision, onClose }: {
  workspaceCode: string; resourceCode: string; fileName: string; kind: ResourceKind;
  revision: string; onClose: () => void;
}) {
  const { t } = useTranslation("pages");
  const [fullscreen, setFullscreen] = useState(false);
  const fullscreenLabel = t(`resourceEditor.preview.${fullscreen ? "exitFullscreen" : "fullscreen"}`);
  return <Dialog open onOpenChange={(open, details) => {
    if (!open && fullscreen && details.reason === "escape-key") {
      details.cancel();
      setFullscreen(false);
    } else if (!open) onClose();
  }}>
    <DialogContent
      showCloseButton={false}
      aria-describedby={undefined}
      className={fullscreen
        ? "inset-0 flex h-dvh max-h-dvh w-full max-w-none translate-x-0 translate-y-0 flex-col overflow-hidden rounded-none sm:max-w-none"
        : "flex max-h-[90vh] flex-col overflow-hidden sm:max-w-5xl"}
    >
      <DialogHeader className="flex-row items-center justify-between gap-3">
        <DialogTitle className="min-w-0 break-words">{fileName}</DialogTitle>
        <div className="flex shrink-0 items-center gap-2">
          <Button variant="ghost" size="sm" onClick={() => setFullscreen(current => !current)} aria-label={fullscreenLabel}>
            {fullscreen ? <Minimize2 size={16} aria-hidden="true" /> : <Maximize2 size={16} aria-hidden="true" />}
            {fullscreenLabel}
          </Button>
          <DialogClose render={<Button variant="ghost" size="icon-sm" aria-label={t("resourceEditor.preview.close")} />}>
            <X size={16} aria-hidden="true" />
          </DialogClose>
        </div>
      </DialogHeader>
      <div className={fullscreen ? "min-h-0 flex-1" : "min-h-0 overflow-auto"}>
        <ResourceFilePreview workspaceCode={workspaceCode} resourceCode={resourceCode} fileName={fileName} kind={kind} revision={revision} fullscreen={fullscreen} />
      </div>
    </DialogContent>
  </Dialog>;
}
