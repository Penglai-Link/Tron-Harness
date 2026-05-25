import {
  useEffect,
  useRef,
  type RefObject,
} from "react";

interface DismissibleLayerOptions {
  dismissOnViewportChange?: boolean;
  open: boolean;
  onDismiss: () => void;
  popupRef: RefObject<HTMLElement | null>;
  scrollGracePeriodMs?: number;
  triggerElement?: HTMLElement | null;
}

export function useDismissibleLayer({
  dismissOnViewportChange = true,
  open,
  onDismiss,
  popupRef,
  scrollGracePeriodMs = 150,
  triggerElement,
}: DismissibleLayerOptions) {
  const openedAtRef = useRef(0);

  useEffect(() => {
    if (!open) return;
    openedAtRef.current = Date.now();
  }, [open]);

  useEffect(() => {
    if (!open) return;

    const dismiss = () => onDismiss();
    const withinGracePeriod = () => Date.now() - openedAtRef.current < scrollGracePeriodMs;

    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (popupRef.current?.contains(target)) return;
      if (triggerElement?.contains(target)) return;
      dismiss();
    };

    const handleViewportChange = () => {
      if (withinGracePeriod()) return;
      dismiss();
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") dismiss();
    };

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);

    if (dismissOnViewportChange) {
      window.addEventListener("resize", handleViewportChange);
      window.addEventListener("scroll", handleViewportChange, true);
    }

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);

      if (dismissOnViewportChange) {
        window.removeEventListener("resize", handleViewportChange);
        window.removeEventListener("scroll", handleViewportChange, true);
      }
    };
  }, [
    dismissOnViewportChange,
    onDismiss,
    open,
    popupRef,
    scrollGracePeriodMs,
    triggerElement,
  ]);
}
