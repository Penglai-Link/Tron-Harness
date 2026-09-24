import { useEffect } from "react";
import { createPortal } from "react-dom";
import { CircleAlert } from "lucide-react";

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
  pending = false,
  title,
  tone = "danger",
}: ConfirmDialogProps) {
  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !pending) {
        onCancel();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onCancel, open, pending]);

  if (!open) return null;

  return createPortal(
    <div
      className="fixed inset-0 z-[110] flex items-center justify-center bg-[var(--tron-overlay-bg,var(--color-backdrop-scrim))] p-4 backdrop-blur-sm"
      onMouseDown={(event) => {
        if (event.target !== event.currentTarget || pending) return;
        onCancel();
      }}
    >
      <div
        className="w-[min(92vw,32rem)] rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel ?? title}
      >
        <div className="flex items-start gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
          {tone === "danger" ? (
            <div className="mt-0.5 rounded-full bg-[var(--color-error-container)] p-2 text-[var(--color-on-error-container)]">
              <CircleAlert size={18} aria-hidden="true" />
            </div>
          ) : null}
          <div className="min-w-0 flex-1">
            <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{title}</h2>
            {description ? (
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{description}</p>
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
            onClick={onConfirm}
          >
            {confirmText}
          </Button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
