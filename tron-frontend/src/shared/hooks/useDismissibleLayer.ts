import {
  useEffect,
  useRef,
  type RefObject,
} from "react";

interface DismissibleLayerOptions {
  dismissOnViewportChange?: boolean;
  nestedLayerSelectors?: string[];
  open: boolean;
  onDismiss: () => void;
  popupRef: RefObject<HTMLElement | null>;
  scrollGracePeriodMs?: number;
  triggerElement?: HTMLElement | null;
}

const defaultNestedLayerSelectors = [
  "[data-slot='select-positioner']",
];

export function useDismissibleLayer({
  dismissOnViewportChange = true,
  nestedLayerSelectors = defaultNestedLayerSelectors,
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
    const isWithinLayer = (target: EventTarget | null) => {
      if (!(target instanceof Node)) return false;
      if (popupRef.current?.contains(target)) return true;
      if (triggerElement?.contains(target)) return true;
      if (!(target instanceof Element)) return false;
      return nestedLayerSelectors.some((selector) => Boolean(target.closest(selector)));
    };

    const handlePointerDown = (event: MouseEvent) => {
      if (isWithinLayer(event.target)) return;
      dismiss();
    };

    const handleViewportChange = (event?: Event) => {
      if (withinGracePeriod()) return;
      if (event && isWithinLayer(event.target)) return;
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
    nestedLayerSelectors,
    onDismiss,
    open,
    popupRef,
    scrollGracePeriodMs,
    triggerElement,
  ]);
}
