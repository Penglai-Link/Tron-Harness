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

function mapMenu(menu: NavigationRuntimeMenu, locale: string): TronNavItem {
  const href = menu.menuType === "ExternalLink"
    ? menu.url ?? "#"
    : menu.menuType === "PackPage"
      ? `/capability-pages/${(menu.pageCode ?? "").split("/").map(encodeURIComponent).join("/")}`
    : menu.menuType === "Internal"
      ? navigationPageRegistry[menu.pageCode ?? ""] ?? "#"
      : menu.menuType === "Widget"
        ? "#"
        : "";

  return {
    menuCode: menu.menuCode,
    isGroup: menu.menuType === "Group",
    widgetCode: menu.menuType === "Widget" ? menu.pageCode ?? undefined : undefined,
    children: menu.children.length ? menu.children.map((child) => mapMenu(child, locale)) : undefined,
    external: menu.menuType === "ExternalLink",
    href,
    icon: getNavigationIcon(menu.icon) ?? Folder,
    label: menu.menuCode === "DUDES"
      ? localizedDudeLabel(menu.label, locale)
      : menu.label,
    childDisplay: menu.childDisplay,
    translationKey: "",
  };
}

function localizedDudeLabel(label: string, locale: string): string {
  const legacyBilingualLabel = /^(.*?)（([^）]*)）$/u.exec(label);
  if (!legacyBilingualLabel) return label;
  return locale.toLowerCase().startsWith("zh")
    ? legacyBilingualLabel[2]
    : legacyBilingualLabel[1];
}

export function mapNavigationTree(tree: NavigationRuntimeTree): TronNavSection[] {
  const prune = (menus: NavigationRuntimeMenu[]): NavigationRuntimeMenu[] => menus.flatMap((menu) => {
    if (menu.pageCode?.startsWith("observability.") || menu.menuCode === "OBSERVABILITY") return [];
    // Persisted menu trees may still reference pages removed from this build.
    if (menu.menuType === "Internal" && !Object.hasOwn(navigationPageRegistry, menu.pageCode ?? "")) return [];
    const children = prune(menu.children);
    if (menu.children.length && !children.length && menu.menuType === "Group") return [];
    return [{ ...menu, children }];
  });
  return prune(tree.menus).map((menu) => ({
    id: menu.menuCode,
    standalone: menu.menuType !== "Group" && menu.children.length === 0,
    flyout: menu.menuType === "Group" && menu.childDisplay === "Flyout",
    icon: getNavigationIcon(menu.icon) ?? Folder,
    items: menu.menuType === "Group"
      ? menu.children.map((child) => mapMenu(child, tree.locale))
      : [mapMenu(menu, tree.locale)],
    label: menu.label,
    translationKey: "",
  }));
}
