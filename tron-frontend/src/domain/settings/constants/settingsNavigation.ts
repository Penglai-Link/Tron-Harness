import {
  Archive,
  Palette,
  Bot,
  Boxes,
  KeyRound,
  ListTree,
  Keyboard,
  Network,
  PanelTop,
  Package,
  MessageSquareText,
  Settings2,
  Sparkles,
  Wrench,
} from "lucide-react";

export const settingsNavigation = [
  {
    groupKey: "foundation",
    label: "Foundation",
    translationKey: "shell:settings.groups.foundation",
    items: [
      {
        label: "General",
        href: "/settings/general",
        icon: Settings2,
        translationKey: "shell:settings.items.general",
      },
      {
        label: "Appearance",
        href: "/settings/appearance",
        icon: Palette,
        translationKey: "shell:settings.items.appearance",
      },
      {
        label: "Shortcuts",
        href: "/settings/shortcuts",
        icon: Keyboard,
        translationKey: "shell:settings.items.shortcuts",
      },
    ],
  },
  {
    groupKey: "platform",
    label: "Platform",
    translationKey: "shell:settings.groups.platform",
    items: [
      {
        label: "Model Providers",
        href: "/settings/model-providers",
        icon: Sparkles,
        translationKey: "shell:settings.items.modelProviders",
      },
      {
        label: "Models",
        href: "/settings/models",
        icon: Sparkles,
        translationKey: "shell:settings.items.models",
      },
      {
        label: "Prompts",
        href: "/settings/prompts",
        icon: MessageSquareText,
        translationKey: "shell:settings.items.prompts",
      },
      {
        label: "Resources",
        href: "/settings/resources",
        icon: Archive,
        translationKey: "shell:settings.items.resources",
      },
      {
        label: "Agents",
        href: "/settings/agents",
        icon: Bot,
        translationKey: "shell:settings.items.agents",
      },
      {
        label: "Tools",
        href: "/settings/tools",
        icon: Wrench,
        translationKey: "shell:settings.items.tools",
      },
      {
        label: "Public Variables",
        href: "/settings/variables",
        icon: KeyRound,
        translationKey: "shell:settings.items.variables",
      },
    ],
  },
  {
    groupKey: "registry",
    label: "Registry",
    translationKey: "shell:settings.groups.registry",
    items: [
      {
        label: "Memory Domains",
        href: "/settings/memory-domains",
        icon: Boxes,
        translationKey: "shell:settings.items.memoryDomains",
      },
      {
        label: "Skill Center",
        href: "/settings/skills",
        icon: Package,
        translationKey: "shell:settings.items.skillCenter",
      },
    ],
  },
  {
    groupKey: "integration",
    label: "Integration",
    translationKey: "shell:settings.groups.integration",
    items: [
      {
        label: "MCP Servers",
        href: "/settings/mcp-servers",
        icon: Network,
        translationKey: "shell:settings.items.mcpServers",
      },
    ],
  },
  {
    groupKey: "administration",
    label: "Administration",
    translationKey: "shell:settings.groups.administration",
    items: [
      {
        label: "Workspace",
        href: "/admin/workspace",
        icon: Settings2,
        translationKey: "shell:settings.items.workspaceAdmin",
      },
      {
        label: "Navigation Domains",
        href: "/admin/navigation/domains",
        icon: Network,
        translationKey: "shell:settings.items.navigationDomains",
      },
      {
        label: "Menu Pool",
        href: "/admin/navigation/menus",
        icon: PanelTop,
        translationKey: "shell:settings.items.menuPool",
      },
      {
        label: "Menu Arrangement",
        href: "/admin/navigation/menu-tree",
        icon: ListTree,
        translationKey: "shell:settings.items.menuArrangement",
      },
    ],
  },
] as const;
