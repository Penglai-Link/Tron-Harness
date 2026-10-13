import {
  Bot,
  MessageSquareText,
  SquarePen,
  FolderKanban,
  History,
  Ellipsis,
  Bell,
  ScrollText,
  Activity,
  LayoutDashboard,
  ChartNoAxesCombined,
  AlarmClock,
  Archive,
  Database,
  DatabaseZap,
  Folder,
  FolderGit2,
  KeyRound,
  ListTodo,
  PlugZap,
  Server,
  Sparkles,
  UserRound,
  Wrench,
} from "lucide-react";

import type { TronNavSection } from "@/shared/types/ShellNavigation";

export const tronNavigation: TronNavSection[] = [
  { id: "EXPLORE_MORE", label: "Explore more", translationKey: "shell:nav.sections.exploreMore", icon: Ellipsis, flyout: true, items: [] },
  {
    id: "CHAT_SESSIONS",
    label: "Chat",
    translationKey: "shell:nav.sections.chat",
    icon: MessageSquareText,
    items: [
      { menuCode: "CHAT_NEW_SESSION", widgetCode: "chat.newSession", label: "New session", href: "", icon: SquarePen, translationKey: "shell:chatSidebar.newChat" },
      { menuCode: "CHAT_PROJECTS", widgetCode: "chat.projects", label: "Projects", href: "", icon: FolderKanban, translationKey: "shell:nav.items.projects" },
      { menuCode: "CHAT_RECENTS", widgetCode: "chat.recents", label: "Recents", href: "", icon: History, translationKey: "shell:chatSidebar.recent" },
    ],
  },
  {
    label: "Core",
    translationKey: "shell:nav.sections.core",
    items: [
      { label: "Dude", href: "/dudes", icon: Bot, translationKey: "shell:nav.items.dudes" },
      { label: "Notifications", href: "/notifications", icon: Bell, translationKey: "shell:nav.items.notifications" },
      {
        label: "Apps",
        href: "/apps",
        icon: FolderGit2,
        translationKey: "shell:nav.items.apps",
      },
      {
        label: "Tasks",
        href: "/tasks",
        icon: ListTodo,
        translationKey: "shell:nav.items.tasks",
      },
      {
        label: "Automations",
        href: "/automations",
        icon: AlarmClock,
        translationKey: "shell:nav.items.automations",
      },
    ],
  },
  {
    label: "Agent Operations",
    translationKey: "shell:nav.sections.agentOperations",
    items: [
      {
        label: "MCP Servers",
        href: "/mcp-servers",
        icon: PlugZap,
        translationKey: "shell:nav.items.mcpServers",
      },
      {
        label: "Models",
        href: "/models",
        icon: Sparkles,
        translationKey: "shell:nav.items.models",
      },
    ],
  },
  {
    label: "Data",
    translationKey: "shell:nav.sections.data",
    items: [
      {
        label: "Resource Management",
        href: "/resource-management/files",
        icon: Archive,
        translationKey: "shell:nav.items.resourceManagement",
        children: [
          {
            label: "File Management",
            href: "/resource-management/files",
            icon: Folder,
            translationKey: "shell:nav.items.fileManagement",
          },
          {
            label: "Data Source Management",
            href: "/resource-management/data-sources",
            icon: DatabaseZap,
            translationKey: "shell:nav.items.dataSourceManagement",
          },
          {
            label: "Server Management",
            href: "/resource-management/servers",
            icon: Server,
            translationKey: "shell:nav.items.serverManagement",
          },
          {
            label: "Server Account Management",
            href: "/resource-management/server-accounts",
            icon: UserRound,
            translationKey: "shell:nav.items.serverAccountManagement",
          },
        ],
      },
      {
        label: "Variables",
        href: "/variables",
        icon: KeyRound,
        translationKey: "shell:nav.items.variables",
      },
    ],
  },

];
