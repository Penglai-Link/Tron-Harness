import {
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
  ScrollText,
  Sparkles,
  UserRound,
  Wrench,
} from "lucide-react";

import type { TronNavSection } from "@/shared/types/ShellNavigation";

export const tronNavigation: TronNavSection[] = [
  {
    label: "Core",
    translationKey: "shell:nav.sections.core",
    items: [
      { label: "Apps", href: "/apps", icon: FolderGit2, translationKey: "shell:nav.items.apps" },
      { label: "Tasks", href: "/tasks", icon: ListTodo, translationKey: "shell:nav.items.tasks" },
      { label: "Automations", href: "/automations", icon: AlarmClock, translationKey: "shell:nav.items.automations" },
    ],
  },
  {
    label: "Agent Operations",
    translationKey: "shell:nav.sections.agentOperations",
    items: [
      { label: "MCP Servers", href: "/mcp-servers", icon: PlugZap, translationKey: "shell:nav.items.mcpServers" },
      { label: "Prompts", href: "/prompts", icon: ScrollText, translationKey: "shell:nav.items.prompts" },
      { label: "Models", href: "/models", icon: Sparkles, translationKey: "shell:nav.items.models" },
      { label: "Skills", href: "/skills", icon: Wrench, translationKey: "shell:nav.items.skills" },
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
          { label: "File Management", href: "/resource-management/files", icon: Folder, translationKey: "shell:nav.items.fileManagement" },
          { label: "Data Source Management", href: "/resource-management/data-sources", icon: DatabaseZap, translationKey: "shell:nav.items.dataSourceManagement" },
          { label: "Server Management", href: "/resource-management/servers", icon: Server, translationKey: "shell:nav.items.serverManagement" },
          { label: "Server Account Management", href: "/resource-management/server-accounts", icon: UserRound, translationKey: "shell:nav.items.serverAccountManagement" },
        ],
      },
      { label: "Variables", href: "/variables", icon: KeyRound, translationKey: "shell:nav.items.variables" },
    ],
  },
];
