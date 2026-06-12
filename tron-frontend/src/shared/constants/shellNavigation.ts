import {
  AlarmClock,
  Archive,
  Bot,
  BookOpenText,
  Database,
  FileClock,
  FolderGit2,
  Gauge,
  KeyRound,
  LayoutDashboard,
  LockKeyhole,
  MessageSquareText,
  Network,
  ListTodo,
  ScrollText,
  Search,
  Settings2,
  Sparkles,
  UsersRound,
  Wrench,
} from "lucide-react";

import type { TronNavSection } from "@/shared/types/ShellNavigation";

export const tronNavigation: TronNavSection[] = [
  {
    label: "Core",
    translationKey: "shell:nav.sections.core",
    items: [
      { label: "Dashboard", href: "/dashboard", icon: LayoutDashboard, translationKey: "shell:nav.items.dashboard" },
      { label: "Apps", href: "/apps", icon: FolderGit2, translationKey: "shell:nav.items.apps" },
      { label: "Tasks", href: "/tasks", icon: ListTodo, translationKey: "shell:nav.items.tasks" },
      { label: "Automations", href: "/automations", icon: AlarmClock, translationKey: "shell:nav.items.automations" },
    ],
  },
  {
    label: "Agent Operations",
    translationKey: "shell:nav.sections.agentOperations",
    items: [
      { label: "Chats", href: "/chats", icon: MessageSquareText, translationKey: "shell:nav.items.chats" },
      { label: "Agents", href: "/agents", icon: Bot, translationKey: "shell:nav.items.agents" },
      { label: "MCP Servers", href: "/mcp-servers", icon: Network, translationKey: "shell:nav.items.mcpServers" },
      { label: "Prompts", href: "/prompts", icon: ScrollText, translationKey: "shell:nav.items.prompts" },
      { label: "Models", href: "/models", icon: Sparkles, translationKey: "shell:nav.items.models" },
      { label: "Skills", href: "/skills", icon: Wrench, translationKey: "shell:nav.items.skills" },
    ],
  },
  {
    label: "Data",
    translationKey: "shell:nav.sections.data",
    items: [
      { label: "Memory", href: "/memory", icon: Database, translationKey: "shell:nav.items.memory" },
      { label: "KnowledgeGraph", href: "/knowledge-graph", icon: Network, translationKey: "shell:nav.items.knowledgeGraph" },
      { label: "KnowledgeBase", href: "/knowledge-base", icon: BookOpenText, translationKey: "shell:nav.items.knowledgeBase" },
      { label: "RAG", href: "/rag", icon: Search, translationKey: "shell:nav.items.rag" },
      { label: "Resources", href: "/resources", icon: Archive, translationKey: "shell:nav.items.resources" },
      { label: "Variables", href: "/variables", icon: KeyRound, translationKey: "shell:nav.items.variables" },
      { label: "Observability", href: "/observability", icon: Gauge, translationKey: "shell:nav.items.observability" },
    ],
  },
  {
    label: "Administration",
    requiredRoleCodes: ["ADMIN"],
    translationKey: "shell:nav.sections.administration",
    items: [
      { label: "Users", href: "/admin/users", icon: UsersRound, translationKey: "shell:nav.items.users" },
      { label: "Roles", href: "/admin/roles", icon: LockKeyhole, translationKey: "shell:nav.items.roles" },
      { label: "Audit Logs", href: "/admin/audit-logs", icon: FileClock, translationKey: "shell:nav.items.auditLogs" },
      { label: "Workspace", href: "/admin/workspace", icon: Settings2, translationKey: "shell:nav.items.workspaceAdmin" },
    ],
  },
];
