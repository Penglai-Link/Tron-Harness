import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export const BUILTIN_AGENT_AVATAR_IDS = ["robot", "fox", "cat", "owl", "astronaut", "wizard", "panda", "frog", "penguin", "raccoon", "rabbit", "dragon"] as const;
const assets = import.meta.glob<string>("../assets/avatars/*.png", { eager: true, query: "?url", import: "default" });

export function AgentAvatar({ avatarId, className }: { avatarId?: string; className?: string }) {
  const id = BUILTIN_AGENT_AVATAR_IDS.find((id) => id === avatarId) ?? "robot";
  return <img src={assets[`../assets/avatars/${id}.png`]} alt="" width={64} height={64} draggable={false} className={cn("object-cover [image-rendering:pixelated]", className)} />;
}

export function AgentAvatarPicker({ value, disabled, onChange }: { value: string; disabled?: boolean; onChange: (id: string) => void }) {
  const { t } = useTranslation("pages");
  const label = t("workspaceAgents.internal.avatar.label", { defaultValue: "Avatar" });
  return <fieldset disabled={disabled} className="space-y-2">
    <legend className="text-sm font-medium">{label}</legend>
    <div className="grid grid-cols-6 gap-2 sm:grid-cols-12" role="group" aria-label={label}>
      {BUILTIN_AGENT_AVATAR_IDS.map((id) => <Button key={id} type="button" variant="outline" size="icon" aria-pressed={value === id}
        aria-label={t(`workspaceAgents.internal.avatar.${id}`)} title={t(`workspaceAgents.internal.avatar.${id}`)}
        className={cn("relative size-auto aspect-square overflow-hidden rounded-lg p-0 transition-transform motion-safe:hover:-translate-y-0.5 motion-reduce:transition-none", value === id && "ring-2 ring-[var(--color-focus-ring,var(--color-primary))] ring-offset-2 ring-offset-[var(--color-surface)]")}
        onClick={() => onChange(id)}>
        <AgentAvatar avatarId={id} className="size-full" />
        {value === id ? <span className="absolute bottom-0 right-0 rounded-tl bg-[var(--color-primary)] p-0.5 text-[var(--color-on-primary)]"><Check size={10} aria-hidden="true" /></span> : null}
      </Button>)}
    </div>
  </fieldset>;
}
