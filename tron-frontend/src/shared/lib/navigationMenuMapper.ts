import { createElement, forwardRef } from "react";
import { Folder, LoaderCircle, type LucideIcon, type LucideProps } from "lucide-react";
import { DynamicIcon, iconNames, type IconName } from "lucide-react/dynamic";

import { navigationPageRegistry } from "@/shared/constants/navigationPageRegistry";
import type { TronNavItem, TronNavSection } from "@/shared/types/ShellNavigation";
import type {
  NavigationRuntimeMenu,
  NavigationRuntimeTree,
} from "@/shared/types/Navigation";

const navigationIconNames: Set<string> = new Set(iconNames);
const dynamicNavigationIcons = new Map<IconName, LucideIcon>();

function toLucideIconName(iconName: string): string {
  return iconName
    .trim()
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1-$2")
    .replace(/[\s_]+/g, "-")
    .replace(/-+/g, "-")
    .toLowerCase();
}

export function isNavigationIconName(iconName?: string | null): boolean {
  const normalizedName = iconName?.trim();
  return Boolean(normalizedName && navigationIconNames.has(toLucideIconName(normalizedName)));
}

function createDynamicNavigationIcon(name: IconName): LucideIcon {
  const cachedIcon = dynamicNavigationIcons.get(name);
  if (cachedIcon) {
    return cachedIcon;
  }

  const DynamicNavigationIcon = forwardRef<SVGSVGElement, LucideProps>((props, ref) => createElement(
    DynamicIcon,
    { ...props, fallback: () => createElement(LoaderCircle), name, ref },
  ));
  DynamicNavigationIcon.displayName = `NavigationIcon(${name})`;
  dynamicNavigationIcons.set(name, DynamicNavigationIcon);
  return DynamicNavigationIcon;
}

export function getNavigationIcon(iconName?: string | null): LucideIcon | null {
  const normalizedName = iconName?.trim();
  if (!normalizedName) {
    return null;
  }

  const dynamicName = toLucideIconName(normalizedName);
  return navigationIconNames.has(dynamicName)
    ? createDynamicNavigationIcon(dynamicName as IconName)
    : null;
}

function mapMenu(menu: NavigationRuntimeMenu): TronNavItem {
  const href = menu.menuType === "ExternalLink"
    ? menu.url ?? "#"
    : menu.menuType === "Internal"
      ? navigationPageRegistry[menu.pageCode ?? ""] ?? "#"
      : menu.menuType === "Widget"
        ? "#"
        : "";

  return {
    children: menu.children.length ? menu.children.map(mapMenu) : undefined,
    external: menu.menuType === "ExternalLink",
    href,
    icon: getNavigationIcon(menu.icon) ?? Folder,
    label: menu.label,
    childDisplay: menu.childDisplay,
    translationKey: "",
  };
}

export function mapNavigationTree(tree: NavigationRuntimeTree): TronNavSection[] {
  return tree.menus.map((menu) => ({
    id: menu.menuCode,
    flyout: menu.childDisplay === "Flyout" && menu.children.length > 0,
    icon: getNavigationIcon(menu.icon) ?? Folder,
    items: menu.children.length ? menu.children.map(mapMenu) : [mapMenu(menu)],
    label: menu.label,
    translationKey: "",
  }));
}
