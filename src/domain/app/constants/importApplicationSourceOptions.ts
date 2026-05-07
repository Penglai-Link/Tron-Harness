import {
  Code2,
  FolderOpen,
  Terminal,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

import type { ImportApplicationDefaults } from "@/domain/app/model";

export interface ImportApplicationSourceOption {
  value: ImportApplicationDefaults["source"];
  label: string;
  icon: LucideIcon;
}

export const importApplicationSourceOptions: ImportApplicationSourceOption[] = [
  { value: "github", label: "GitHub", icon: Code2 },
  { value: "gitlab", label: "GitLab", icon: Terminal },
  { value: "local", label: "Local Directory", icon: FolderOpen },
];
