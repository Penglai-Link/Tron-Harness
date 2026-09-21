import type { NavigationRuntimeTree } from "@/shared/types/Navigation";

export const CHAT_PANEL_MENU_TREE_CODE = "CHAT_PANEL";

export const CHAT_PANEL_ROOT_MENU_CODE = "CHAT_SESSIONS";

export const CHAT_PANEL_WIDGET_KEYS = {
  newSession: "chat.newSession",
  projects: "chat.projects",
  recents: "chat.recents",
} as const;

export const DEFAULT_CHAT_PANEL_MENU_TREE: NavigationRuntimeTree = {
  domainCode: CHAT_PANEL_MENU_TREE_CODE,
  locale: "zh-CN",
  maxDepth: 3,
  menus: [
    {
      menuCode: CHAT_PANEL_ROOT_MENU_CODE,
      label: "会话",
      menuType: "Group",
      icon: null,
      pageCode: null,
      url: null,
      childDisplay: "Inline",
      children: [
        {
          menuCode: "CHAT_NEW_SESSION",
          label: "新会话",
          menuType: "Widget",
          icon: null,
          pageCode: CHAT_PANEL_WIDGET_KEYS.newSession,
          url: null,
          childDisplay: "Inline",
          children: [],
        },
        {
          menuCode: "CHAT_PROJECTS",
          label: "项目",
          menuType: "Widget",
          icon: null,
          pageCode: CHAT_PANEL_WIDGET_KEYS.projects,
          url: null,
          childDisplay: "Inline",
          children: [],
        },
        {
          menuCode: "CHAT_RECENTS",
          label: "最近",
          menuType: "Widget",
          icon: null,
          pageCode: CHAT_PANEL_WIDGET_KEYS.recents,
          url: null,
          childDisplay: "Inline",
          children: [],
        },
      ],
    },
  ],
};
