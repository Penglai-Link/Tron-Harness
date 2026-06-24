import * as ResizablePrimitive from "react-resizable-panels"

import { cn } from "@/lib/utils"

function ResizablePanelGroup({
  className,
  ...props
}: ResizablePrimitive.GroupProps) {
  return (
    <ResizablePrimitive.Group
      data-slot="resizable-panel-group"
      className={cn(
        "flex h-full w-full aria-[orientation=vertical]:flex-col",
        className
      )}
      {...props}
    />
  )
}

function ResizablePanel({ ...props }: ResizablePrimitive.PanelProps) {
  return <ResizablePrimitive.Panel data-slot="resizable-panel" {...props} />
}

function ResizableHandle({
  withHandle,
  className,
  ...props
}: ResizablePrimitive.SeparatorProps & {
  withHandle?: boolean
}) {
  return (
    <ResizablePrimitive.Separator
      data-slot="resizable-handle"
      className={cn(
        "group relative flex w-0 shrink-0 items-center justify-center bg-transparent",
        className
      )}
      {...props}
    >
      <div className="pointer-events-none absolute inset-y-0 left-1/2 w-px -translate-x-1/2 hover:bg-[var(--color-outline-variant)] transition-colors group-hover:bg-[var(--color-outline)] group-aria-[orientation=horizontal]:inset-x-0 group-aria-[orientation=horizontal]:inset-y-auto group-aria-[orientation=horizontal]:top-1/2 group-aria-[orientation=horizontal]:h-px group-aria-[orientation=horizontal]:w-full group-aria-[orientation=horizontal]:-translate-y-1/2 group-aria-[orientation=horizontal]:translate-x-0" />
      {withHandle && (
        <div className="z-20 flex h-12 w-2 shrink-0 rounded-lg bg-[var(--color-outline-variant)] transition-colors group-hover:bg-[var(--color-outline)] group-aria-[orientation=horizontal]:h-1 group-aria-[orientation=horizontal]:w-6" />
      )}
    </ResizablePrimitive.Separator>
  )
}

export { ResizableHandle, ResizablePanel, ResizablePanelGroup }
