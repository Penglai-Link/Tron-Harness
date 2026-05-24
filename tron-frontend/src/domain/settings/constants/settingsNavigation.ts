import {
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
] as const;
