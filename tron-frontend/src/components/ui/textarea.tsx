import * as React from "react"

import { cn } from "@/lib/utils"

function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      data-slot="textarea"
      className={cn(
        "tron-input flex field-sizing-content min-h-16 w-full rounded-[var(--tron-input-radius,var(--tron-control-radius,4px))] border px-[var(--tron-input-padding-x,0.625rem)] py-2 text-base transition-colors placeholder:text-[var(--tron-input-placeholder,var(--muted-foreground))] disabled:cursor-not-allowed md:text-sm ",
        className
      )}
      {...props}
    />
  )
}

export { Textarea }
