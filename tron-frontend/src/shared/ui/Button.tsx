import {
  cva,
  type VariantProps,
} from "class-variance-authority";

import type {
  AnchorHTMLAttributes,
  ButtonHTMLAttributes,
} from "react";
import {
  Link,
  type LinkProps,
} from "react-router";

import { cn } from "@/shared/lib/utils";

export const buttonVariants = cva(
  "inline-flex h-10 items-center justify-center gap-[12px] rounded-[4px] px-4 text-sm font-medium tracking-normal transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        primary: "bg-[var(--color-primary)] text-[var(--color-on-primary)] hover:bg-[var(--color-primary-container)]",
        secondary: "bg-[var(--color-surface-container)] text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]",
        ghost: "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
      },
      size: {
        sm: "h-8 px-3 text-[13px]",
        md: "h-10 px-4 text-sm",
        icon: "size-10 px-0",
      },
    },
    defaultVariants: {
      variant: "primary",
      size: "md",
    },
  },
);

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {}

export interface LinkButtonProps extends LinkProps, VariantProps<typeof buttonVariants> {}

export interface AnchorButtonProps extends AnchorHTMLAttributes<HTMLAnchorElement>, VariantProps<typeof buttonVariants> {}

export function Button({ className, size, variant, ...props }: ButtonProps) {
  return <button className={cn(buttonVariants({ className, size, variant }))} {...props} />;
}

export function LinkButton({ className, size, variant, ...props }: LinkButtonProps) {
  return <Link className={cn(buttonVariants({ className, size, variant }))} {...props} />;
}

export function AnchorButton({ className, size, variant, ...props }: AnchorButtonProps) {
  return <a className={cn(buttonVariants({ className, size, variant }))} {...props} />;
}
