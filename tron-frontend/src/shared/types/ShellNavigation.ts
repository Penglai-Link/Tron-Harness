import type { LucideIcon } from "lucide-react";

export interface TronNavItem {
  children?: TronNavItem[];
  label: string;
  href: string;
  icon: LucideIcon;
  translationKey: string;
}

export interface TronNavSection {
  label: string;
  items: TronNavItem[];
  requiredRoleCodes?: string[];
  translationKey: string;
}
