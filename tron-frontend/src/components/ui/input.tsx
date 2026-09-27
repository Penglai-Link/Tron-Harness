import * as React from "react"
import { Input as InputPrimitive } from "@base-ui/react/input"

import { cn } from "@/lib/utils"

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <InputPrimitive
      type={type}
      data-slot="input"
      className={cn(
        "tron-input h-[var(--tron-input-height,2rem)] w-full min-w-0 rounded-[var(--tron-input-radius,var(--tron-control-radius,4px))] border px-[var(--tron-input-padding-x,0.625rem)] py-1 text-base transition-colors file:inline-flex file:h-6 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-[var(--tron-input-placeholder,var(--muted-foreground))] disabled:pointer-events-none disabled:cursor-not-allowed md:text-sm ",
        className
      )}
      {...props}
      {...(props["aria-label"] && !props["aria-labelledby"] ? { "aria-labelledby": "" } : {})}
    />
  )
}

export { Input }
