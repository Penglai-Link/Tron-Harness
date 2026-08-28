import { X } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";

import {
  PageView,
  type TargetPageFrameInspectionResult,
} from "./Index";

interface LinkBrowserDialogProps {
  onClose: () => void;
  onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
  url: string;
}

function displayHost(url: string): string {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}

export function LinkBrowserDialog({
  onClose,
  onInspectionConfirm,
  url,
}: LinkBrowserDialogProps) {
  const { t } = useTranslation("pages");

  return (
    <Dialog open onOpenChange={(open) => {
      if (!open) {
        onClose();
      }
    }}>
      <DialogContent
        showCloseButton={false}
        className="flex h-[calc(100vh-4rem)] w-[calc(100vw-4rem)] max-w-none flex-col gap-0 overflow-hidden p-0 sm:max-w-none"
      >
        <DialogTitle className="sr-only">
          {t("chat.markdownLink.browserTitle")}
        </DialogTitle>
        <header className="flex h-12 shrink-0 items-center justify-between border-b border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface-strong)] px-4">
          <span className="min-w-0 truncate text-sm text-[var(--color-on-surface)]">
            {displayHost(url)}
          </span>
          <DialogClose
            render={
              <button
                type="button"
                aria-label={t("chat.markdownLink.browserClose")}
                className="inline-flex size-8 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]"
              />
            }
          >
            <X size={18} aria-hidden="true" />
          </DialogClose>
        </header>
        <div className="min-h-0 flex-1">
          <PageView
            showChrome={false}
            url={url}
            onInspectionConfirm={onInspectionConfirm}
          />
        </div>
      </DialogContent>
    </Dialog>
  );
}
