import type { LucideIcon } from "lucide-react";

export interface TronNavItem {
  children?: TronNavItem[];
  label: string;
  href: string;
  icon: LucideIcon;
  translationKey: string;
  external?: boolean;
}

export interface TronNavSection {
  id?: string;
  label: string;
  items: TronNavItem[];
  requiredRoleCodes?: string[];
  translationKey: string;
}
