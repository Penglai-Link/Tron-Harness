export type NavigationMenuType = "Group" | "Internal" | "ExternalLink" | "Widget" | "PackPage";
export type NavigationMenuSource = "BuiltIn" | "Custom" | "CapabilityPack";
export type NavigationMenuChildDisplay = "Inline" | "Flyout";

export interface NavigationLocalizedText {
  "zh-CN": string;
  "en-US": string;
}

export interface NavigationDomainOption {
  domainCode: string;
  name: string;
  description: string;
  builtIn: boolean;
  isDefault: boolean;
  position: number;
}

export interface NavigationRuntimeMenu {
  menuCode: string;
  label: string;
  menuType: NavigationMenuType;
  icon: string | null;
  pageCode: string | null;
  url: string | null;
  childDisplay: NavigationMenuChildDisplay;
  children: NavigationRuntimeMenu[];
}

export interface NavigationRuntimeTree {
  domainCode: string;
  locale: string;
  maxDepth: number;
  menus: NavigationRuntimeMenu[];
}

export interface NavigationAdminDomain {
  domainCode: string;
  providerPackId: string | null;
  name: NavigationLocalizedText;
  description: NavigationLocalizedText;
  builtIn: boolean;
  enabled: boolean;
  isDefault: boolean;
  position: number;
  createdAt: number;
  updatedAt: number;
}

export type NavigationMenuTreeKind = "Domain";

export interface NavigationAdminMenuTree {
  treeCode: string;
  treeKind: NavigationMenuTreeKind;
  name: NavigationLocalizedText;
  description: NavigationLocalizedText;
  builtIn: boolean;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface NavigationAdminMenu {
  menuCode: string;
  source: NavigationMenuSource;
  menuType: NavigationMenuType;
  translations: NavigationLocalizedText;
  description: NavigationLocalizedText;
  pageCode: string | null;
  url: string | null;
  icon: string | null;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface NavigationAdminTreeItem extends NavigationAdminMenu {
  parentMenuCode: string | null;
  position: number;
  childDisplay: NavigationMenuChildDisplay;
  children: NavigationAdminTreeItem[];
}

export interface NavigationAdminPage<T> {
  current: number;
  size: number;
  total: number;
  records: T[];
}
