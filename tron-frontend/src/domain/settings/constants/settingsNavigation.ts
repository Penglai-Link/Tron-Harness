import {
  Bot,
  KeyRound,
  Network,
  Package,
  Settings2,
  Sparkles,
  Wrench,
} from "lucide-react";

export const settingsNavigation = [
  {
    label: "General",
    href: "/settings/general",
    icon: Settings2,
    translationKey: "shell:settings.items.general",
  },
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
  {
    label: "MCP Servers",
    href: "/settings/mcp-servers",
    icon: Network,
    translationKey: "shell:settings.items.mcpServers",
  },
  {
    label: "Variables",
    href: "/settings/variables",
    icon: KeyRound,
    translationKey: "shell:settings.items.variables",
  },
  {
    label: "Skill Center",
    href: "/settings/skills",
    icon: Package,
    translationKey: "shell:settings.items.skillCenter",
  },
] as const;
