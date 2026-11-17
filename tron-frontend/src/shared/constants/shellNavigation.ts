import { Folder } from "lucide-react";
import { getNavigationIcon } from "@/shared/lib/navigationMenuMapper";
import type { TronNavSection } from "@/shared/types/ShellNavigation";

// Mirrors the built-in navigation seed; API trees remain authoritative.
export const tronNavigation: TronNavSection[] = [
  { id: "DUDES", label: "Dude", translationKey: "shell:nav.items.dudes", icon: getNavigationIcon("Bot") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "DUDES", label: "Dude", translationKey: "shell:nav.items.dudes", href: "/dudes", icon: getNavigationIcon("Bot") ?? Folder, childDisplay: "Inline" }
] },
  { id: "TASKS", label: "Tasks", translationKey: "shell:nav.items.tasks", icon: getNavigationIcon("ListTodo") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "TASKS", label: "Tasks", translationKey: "shell:nav.items.tasks", href: "/tasks", icon: getNavigationIcon("ListTodo") ?? Folder, childDisplay: "Inline" }
] },
  { id: "AUTOMATIONS", label: "Automations", translationKey: "shell:nav.items.automations", icon: getNavigationIcon("AlarmClock") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "AUTOMATIONS", label: "Automations", translationKey: "shell:nav.items.automations", href: "/automations", icon: getNavigationIcon("AlarmClock") ?? Folder, childDisplay: "Inline" }
] },
  { id: "EXPLORE_MORE", label: "Explore more", translationKey: "shell:nav.sections.exploreMore", icon: getNavigationIcon("Ellipsis") ?? Folder, standalone: false, flyout: true, items: [
{ menuCode: "APPS", label: "Apps", translationKey: "shell:nav.items.apps", href: "/apps", icon: getNavigationIcon("FolderGit2") ?? Folder, childDisplay: "Inline" },
{ menuCode: "NOTIFICATIONS", label: "Notifications", translationKey: "shell:nav.items.notifications", href: "/notifications", icon: getNavigationIcon("Bell") ?? Folder, childDisplay: "Inline" },
{ menuCode: "VARIABLES", label: "Variables", translationKey: "shell:nav.items.variables", href: "/variables", icon: getNavigationIcon("KeyRound") ?? Folder, childDisplay: "Inline" },
{ menuCode: "MCP_SERVERS", label: "MCP Servers", translationKey: "shell:nav.items.mcpServers", href: "/mcp-servers", icon: getNavigationIcon("PlugZap") ?? Folder, childDisplay: "Inline" },
{ menuCode: "FILE_MANAGEMENT", label: "File Management", translationKey: "shell:nav.items.fileManagement", href: "/resource-management/files", icon: getNavigationIcon("Folder") ?? Folder, childDisplay: "Inline" },
] },
  { id: "CHAT_PROJECTS", label: "Work Topics", translationKey: "shell:nav.items.projects", icon: getNavigationIcon("FolderKanban") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "CHAT_PROJECTS", label: "Work Topics", translationKey: "shell:nav.items.projects", href: "", icon: getNavigationIcon("FolderKanban") ?? Folder, childDisplay: "Inline", widgetCode: "chat.projects" }
] },
  { id: "CHAT_DIRECTORIES", label: "Projects", translationKey: "shell:chatSidebar.directories", icon: getNavigationIcon("Folder") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "CHAT_DIRECTORIES", label: "Projects", translationKey: "shell:chatSidebar.directories", href: "", icon: getNavigationIcon("Folder") ?? Folder, childDisplay: "Inline", widgetCode: "chat.directories" }
] },
  { id: "CHAT_RECENTS", label: "Quick Chats", translationKey: "shell:chatSidebar.recent", icon: getNavigationIcon("History") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "CHAT_RECENTS", label: "Quick Chats", translationKey: "shell:chatSidebar.recent", href: "", icon: getNavigationIcon("History") ?? Folder, childDisplay: "Inline", widgetCode: "chat.recents" }
] },
];
