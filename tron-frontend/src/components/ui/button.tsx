import { Button as ButtonPrimitive } from "@base-ui/react/button"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const buttonVariants = cva(
  "tron-button group/button inline-flex shrink-0 items-center justify-center bg-clip-padding whitespace-nowrap transition-all select-none active:not-aria-[haspopup]:translate-y-px disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "tron-button-default",
        outline: "tron-button-outline",
        secondary: "tron-button-secondary",
        ghost: "tron-button-ghost",
        destructive: "tron-button-destructive",
        link: "tron-button-link",
      },
      size: {
        default:
          "h-[var(--tron-button-height,2rem)] gap-[var(--tron-button-gap,0.375rem)] px-[var(--tron-button-padding-x,0.625rem)] has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2",
        xs: "h-[var(--tron-button-height-xs,1.5rem)] gap-1 rounded-[var(--tron-button-radius,var(--tron-control-radius,4px))] px-2 text-xs in-data-[slot=button-group]:rounded-[var(--tron-button-radius,var(--tron-control-radius,4px))] has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3",
        sm: "h-[var(--tron-button-height-sm,1.75rem)] gap-1 rounded-[var(--tron-button-radius,var(--tron-control-radius,4px))] px-2.5 text-[0.8rem] in-data-[slot=button-group]:rounded-[var(--tron-button-radius,var(--tron-control-radius,4px))] has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3.5",
        lg: "h-[var(--tron-button-height-lg,2.25rem)] gap-1.5 px-2.5 has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2",
        icon: "size-[var(--tron-button-height,2rem)]",
        "icon-xs":
          "size-[var(--tron-button-height-xs,1.5rem)] rounded-[var(--tron-button-radius,var(--tron-control-radius,4px))] in-data-[slot=button-group]:rounded-[var(--tron-button-radius,var(--tron-control-radius,4px))] [&_svg:not([class*='size-'])]:size-3",
        "icon-sm":
          "size-[var(--tron-button-height-sm,1.75rem)] rounded-[var(--tron-button-radius,var(--tron-control-radius,4px))] in-data-[slot=button-group]:rounded-[var(--tron-button-radius,var(--tron-control-radius,4px))]",
        "icon-lg": "size-[var(--tron-button-height-lg,2.25rem)]",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

function Button({
  className,
  variant = "default",
  size = "default",
  ...props
}: ButtonPrimitive.Props & VariantProps<typeof buttonVariants>) {
  return (
    <ButtonPrimitive
      data-slot="button"
      data-variant={variant}
      data-size={size}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
}

export { Button, buttonVariants }
