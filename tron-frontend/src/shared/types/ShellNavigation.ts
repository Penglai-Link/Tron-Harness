import type { LucideIcon } from "lucide-react";

export interface TronNavItem {
  children?: TronNavItem[];
  label: string;
  href: string;
  icon: LucideIcon;
  translationKey: string;
  external?: boolean;
  childDisplay?: "Inline" | "Flyout";
}

export interface TronNavSection {
  id?: string;
  label: string;
  items: TronNavItem[];
  translationKey: string;
  flyout?: boolean;
  icon?: LucideIcon;
}
