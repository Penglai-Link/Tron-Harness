import type { ComponentProps } from "react";
import { Item } from "@/components/ui/item";
import { cn } from "@/shared/lib/utils";

/** Appearance groups share the workspace surface; only controls and previews have frames. */
export function AppearanceSection({ className, ...props }: ComponentProps<"section">) {
  return <section className={cn(
    "min-w-0 space-y-5 border-b border-[var(--color-outline-variant)]/50 py-6 first:pt-0 last:border-b-0",
    className,
  )} {...props} />;
}

export function AppearanceRow({ className, ...props }: ComponentProps<typeof Item>) {
  return <Item size="xs" className={cn(
    "flex-col items-start gap-3 rounded-none border-0 p-0 @xl/appearance:flex-row @xl/appearance:items-center @xl/appearance:gap-6",
    className,
  )} {...props} />;
}
