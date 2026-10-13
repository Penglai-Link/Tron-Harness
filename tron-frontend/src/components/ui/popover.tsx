import { Popover as PopoverPrimitive } from "@base-ui/react/popover";
import { cn } from "@/shared/lib/utils";

export const Popover = PopoverPrimitive.Root;
export const PopoverTrigger = PopoverPrimitive.Trigger;

export function PopoverContent({ className, side = "bottom", align = "start", sideOffset = 6, ...props }: PopoverPrimitive.Popup.Props & Pick<PopoverPrimitive.Positioner.Props, "side" | "align" | "sideOffset">) {
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Positioner side={side} align={align} sideOffset={sideOffset} className="isolate z-50">
        <PopoverPrimitive.Popup data-slot="popover-content" className={cn("max-h-(--available-height) max-w-[calc(100vw-1rem)] overflow-y-auto rounded-xl bg-[var(--tron-popover-bg,var(--popover))] p-2 text-[var(--tron-popover-fg,var(--popover-foreground))] shadow-lg ring-1 ring-foreground/10 outline-none", className)} {...props} />
      </PopoverPrimitive.Positioner>
    </PopoverPrimitive.Portal>
  );
}
