import type { CSSProperties, ReactNode } from "react";
import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { cn } from "@/shared/lib/utils";

interface InfoTooltipProps {
  align?: "end" | "start";
  ariaLabel: string;
  children: ReactNode;
}

interface TooltipPosition {
  left: number;
  maxHeight: number;
  top: number;
  width: number;
}

const tooltipGap = 8;
const viewportMargin = 16;
const desktopTooltipWidth = 320;

export function InfoTooltip({
  align = "start",
  ariaLabel,
  children,
}: InfoTooltipProps) {
  const tooltipId = useId();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const tooltipRef = useRef<HTMLSpanElement>(null);
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<TooltipPosition>({
    left: viewportMargin,
    maxHeight: 240,
    top: viewportMargin,
    width: desktopTooltipWidth,
  });

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;

    if (!trigger || typeof window === "undefined") {
      return;
    }

    const rect = trigger.getBoundingClientRect();
    const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
    const viewportHeight = document.documentElement.clientHeight || window.innerHeight;
    const mainBounds = trigger.closest("main")?.getBoundingClientRect();
    const boundaryLeft = Math.max(viewportMargin, (mainBounds?.left ?? 0) + viewportMargin);
    const boundaryRight = Math.min(
      viewportWidth - viewportMargin,
      (mainBounds?.right ?? viewportWidth) - viewportMargin,
    );
    const availableWidth = Math.max(0, boundaryRight - boundaryLeft);
    const width = Math.min(desktopTooltipWidth, availableWidth);
    const maxHeight = Math.max(96, viewportHeight - viewportMargin * 2);
    const tooltipHeight = Math.min(tooltipRef.current?.offsetHeight ?? 0, maxHeight);
    const preferredLeft = align === "end" ? rect.right - width : rect.left;
    const left = Math.min(
      Math.max(preferredLeft, boundaryLeft),
      Math.max(boundaryLeft, boundaryRight - width),
    );
    const belowTop = rect.bottom + tooltipGap;
    const aboveTop = rect.top - tooltipHeight - tooltipGap;
    const top = tooltipHeight > 0 && belowTop + tooltipHeight > viewportHeight - viewportMargin
      ? Math.max(viewportMargin, aboveTop)
      : Math.min(belowTop, viewportHeight - viewportMargin);

    setPosition({ left, maxHeight, top, width });
  }, [align]);

  useLayoutEffect(() => {
    if (open) {
      updatePosition();
    }
  }, [open, updatePosition, children]);

  useEffect(() => {
    if (!open) {
      return undefined;
    }

    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);

    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [open, updatePosition]);

  const tooltipStyle: CSSProperties = {
    left: position.left,
    maxHeight: position.maxHeight,
    top: position.top,
    width: position.width,
  };

  const tooltip = typeof document === "undefined" ? null : createPortal(
    <span
      ref={tooltipRef}
      id={tooltipId}
      className={cn(
        "pointer-events-none fixed z-[80] overflow-y-auto rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface-strong)] px-3 py-2 text-left text-[12px] font-normal normal-case leading-5 text-[var(--color-on-surface)] shadow-lg transition-opacity",
        open ? "visible opacity-100" : "invisible opacity-0",
      )}
      role="tooltip"
      style={tooltipStyle}
    >
      <span className="block space-y-2">{children}</span>
    </span>,
    document.body,
  );

  return (
    <span
      className="relative inline-flex items-center align-middle"
      onMouseEnter={() => {
        setOpen(true);
        updatePosition();
      }}
      onMouseLeave={() => setOpen(false)}
    >
      <button
        ref={triggerRef}
        aria-label={ariaLabel}
        aria-describedby={open ? tooltipId : undefined}
        className="inline-flex size-4 items-center justify-center rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] text-[10px] font-bold leading-none text-[var(--color-on-surface-variant)] transition-colors hover:border-[var(--color-outline)] hover:text-[var(--color-on-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-offset-2"
        type="button"
        onBlur={() => setOpen(false)}
        onFocus={() => {
          setOpen(true);
          updatePosition();
        }}
      >
        <span aria-hidden="true">?</span>
      </button>
      {tooltip}
    </span>
  );
}
