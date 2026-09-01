import type { ReactNode } from "react";
import { createPortal } from "react-dom";

import { cn } from "@/shared/lib/utils";

interface ModalDialogProps {
  children: ReactNode;
  className?: string;
  open: boolean;
  title: string;
}

export function ModalDialog({ children, className, open, title }: ModalDialogProps) {
  if (!open) return null;

  return createPortal(
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/45 p-4 backdrop-blur-sm">
      <div
        className={cn(
          "w-full rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]",
          className,
        )}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="border-b border-[var(--color-outline-variant)] px-5 py-4">
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{title}</h2>
        </div>
        <div className="p-5">{children}</div>
      </div>
    </div>,
    document.body,
  );
}
