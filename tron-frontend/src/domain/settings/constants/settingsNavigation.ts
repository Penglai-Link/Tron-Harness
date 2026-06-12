import {
  Bot,
  KeyRound,
  Network,
  Package,
  Settings2,
  ShieldCheck,
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
        label: "Variables",
        href: "/settings/variables",
        icon: KeyRound,
        translationKey: "shell:settings.items.variables",
      },
    ],
  },
  {
    groupKey: "platform",
    label: "Platform",
    translationKey: "shell:settings.groups.platform",
    items: [
      {
        label: "Models",
        href: "/settings/models",
        icon: Sparkles,
        translationKey: "shell:settings.items.models",
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
    ],
  },
  {
    groupKey: "registry",
    label: "Registry",
    translationKey: "shell:settings.groups.registry",
    items: [
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
      {
        label: "OpenAPI",
        href: "/settings/openapi",
        icon: ShieldCheck,
        translationKey: "shell:settings.items.openapi",
      },
    ],
  },
] as const;
