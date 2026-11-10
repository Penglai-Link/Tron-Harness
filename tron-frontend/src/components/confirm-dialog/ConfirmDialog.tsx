import { useRef } from "react";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { CircleAlert, LoaderCircle } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export interface ConfirmDialogProps {
  ariaLabel?: string;
  cancelText?: string;
  confirmText?: string;
  description?: string;
  onCancel: () => void;
  onConfirm: () => void;
  open: boolean;
  pendingText?: string;
  pending?: boolean;
  title: string;
  tone?: "default" | "danger";
}

export function ConfirmDialog({
  ariaLabel,
  cancelText = "Cancel",
  confirmText = "Confirm",
  description,
  onCancel,
  onConfirm,
  open,
  pendingText,
  pending = false,
  title,
  tone = "danger",
}: ConfirmDialogProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  return (
    <Dialog open={open} onOpenChange={(nextOpen) => { if (!nextOpen && !pending) onCancel(); }}>
      <DialogContent
        showCloseButton={false}
        ref={dialogRef}
        initialFocus={dialogRef}
        className="block w-[min(92vw,32rem)] gap-0 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-0 shadow-[0_20px_60px_rgba(0,0,0,0.28)] sm:max-w-[32rem]"
        aria-label={ariaLabel ?? title}
      >
        <div className="flex items-start gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
          {tone === "danger" ? (
            <div className="mt-0.5 rounded-full bg-[var(--color-error-container)] p-2 text-[var(--color-on-error-container)]">
              <CircleAlert size={18} aria-hidden="true" />
            </div>
          ) : null}
          <div className="min-w-0 flex-1">
            <DialogTitle className="text-base font-semibold text-[var(--color-on-surface)]">{title}</DialogTitle>
            {description ? (
              <DialogDescription className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{description}</DialogDescription>
            ) : null}
          </div>
        </div>
        <div className="flex justify-end gap-[12px] px-5 py-4">
          <Button
            type="button"
            variant="ghost"
            disabled={pending}
            onClick={onCancel}
          >
            {cancelText}
          </Button>
          <Button
            type="button"
            className={cn(
              tone === "danger" && "bg-[var(--color-error)] text-[var(--color-on-error)] hover:brightness-95",
            )}
            disabled={pending}
            aria-busy={pending}
            data-dialog-primary
            onClick={onConfirm}
          >
            {pending ? <LoaderCircle className="animate-spin" size={14} aria-hidden="true" /> : null}
            {pending ? pendingText ?? confirmText : confirmText}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
