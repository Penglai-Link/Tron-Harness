import type { ReactNode } from "react";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";

import { cn } from "@/shared/lib/utils";

interface ModalDialogProps {
  children: ReactNode;
  className?: string;
  open: boolean;
  onClose: () => void;
  title: string;
}

export function ModalDialog({ children, className, open, title, onClose }: ModalDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(nextOpen) => { if (!nextOpen) onClose(); }}>
      <DialogContent showCloseButton={false}
        className={cn(
          "block w-full gap-0 p-0 sm:max-w-2xl rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]",
          className,
        )}
        aria-label={title}
      >
        <div className="border-b border-[var(--color-outline-variant)] px-5 py-4">
          <DialogTitle className="text-base font-semibold text-[var(--color-on-surface)]">{title}</DialogTitle>
        </div>
        <div className="p-5">{children}</div>
      </DialogContent>
    </Dialog>
  );
}
