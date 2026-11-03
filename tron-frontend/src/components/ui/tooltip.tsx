"use client"

import { useEffect, useId, useSyncExternalStore } from "react"

import { Tooltip as TooltipPrimitive } from "@base-ui/react/tooltip"

import { cn } from "@/lib/utils"

function TooltipProvider({
  delay = 350,
  ...props
}: TooltipPrimitive.Provider.Props) {
  return (
    <TooltipPrimitive.Provider
      data-slot="tooltip-provider"
      delay={delay}
      {...props}
    />
  )
}

// A single owner also covers independently nested providers and portaled surfaces.
let activeTooltip: string | null = null
const tooltipListeners = new Set<() => void>()
function setActiveTooltip(id: string | null) {
  activeTooltip = id
  tooltipListeners.forEach((listener) => listener())
}
function subscribeTooltip(listener: () => void) {
  tooltipListeners.add(listener)
  return () => { tooltipListeners.delete(listener) }
}

function Tooltip({ onOpenChange, ...props }: Omit<TooltipPrimitive.Root.Props, "open" | "defaultOpen">) {
  const id = useId()
  const active = useSyncExternalStore(subscribeTooltip, () => activeTooltip, () => null)
  useEffect(() => () => {
    if (activeTooltip === id) setActiveTooltip(null)
  }, [id])
  return <TooltipPrimitive.Root {...props} open={active === id}
    onOpenChange={(open, details) => {
      // Focus restoration and dialog autofocus must never open a hint.
      if (open && details.reason !== "trigger-hover") {
        details.cancel()
        return
      }
      onOpenChange?.(open, details)
      if (details.isCanceled) return
      if (open) setActiveTooltip(id)
      else if (activeTooltip === id) setActiveTooltip(null)
    }} />
}

function TooltipTrigger({ delay = 350, closeDelay = 150, ...props }: TooltipPrimitive.Trigger.Props) {
  return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props}
    delay={delay} closeDelay={closeDelay} title={undefined} />
}

function TooltipContent({
  className,
  side = "top",
  sideOffset = 4,
  align = "center",
  alignOffset = 0,
  children,
  ...props
}: TooltipPrimitive.Popup.Props &
  Pick<
    TooltipPrimitive.Positioner.Props,
    "align" | "alignOffset" | "side" | "sideOffset"
  >) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Positioner
        align={align}
        alignOffset={alignOffset}
        side={side}
        sideOffset={sideOffset}
        className="isolate z-[120]"
      >
        <TooltipPrimitive.Popup
          data-slot="tooltip-content"
          role="tooltip"
          className={cn(
            "z-[120] block w-fit max-w-xs origin-(--transform-origin) rounded-md bg-foreground px-3 py-1.5 text-left text-xs leading-relaxed whitespace-normal break-words text-background has-data-[slot=kbd]:pr-1.5 **:data-[slot=kbd]:relative **:data-[slot=kbd]:isolate **:data-[slot=kbd]:z-50 **:data-[slot=kbd]:rounded-sm",
            className
          )}
          {...props}
        >
          {children}
          <TooltipPrimitive.Arrow className="z-50 size-2.5 translate-y-[calc(-50%-2px)] rotate-45 rounded-[2px] bg-foreground fill-foreground data-[side=bottom]:top-1 data-[side=inline-end]:top-1/2! data-[side=inline-end]:-left-1 data-[side=inline-end]:-translate-y-1/2 data-[side=inline-start]:top-1/2! data-[side=inline-start]:-right-1 data-[side=inline-start]:-translate-y-1/2 data-[side=left]:top-1/2! data-[side=left]:-right-1 data-[side=left]:-translate-y-1/2 data-[side=right]:top-1/2! data-[side=right]:-left-1 data-[side=right]:-translate-y-1/2 data-[side=top]:-bottom-2.5" />
        </TooltipPrimitive.Popup>
      </TooltipPrimitive.Positioner>
    </TooltipPrimitive.Portal>
  )
}

export { Tooltip, TooltipTrigger, TooltipContent, TooltipProvider }
