import { Badge as BadgeCom, type BadgeProps } from "@/components/ui/badge"

export function Badge({ className, tone = "default", ...props }: BadgeProps & { tone?: BadgeProps['variant'] }) {
  return (
    <BadgeCom
      {...props}
      variant={tone}
    />
  );
}
