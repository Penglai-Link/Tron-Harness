import {
  KeyRound,
  Package,
  Settings2,
  Sparkles,
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
