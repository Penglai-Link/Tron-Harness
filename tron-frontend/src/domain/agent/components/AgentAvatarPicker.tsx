import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

export const SYSTEM_AGENT_AVATAR_IDS: Readonly<Record<string, string>> = { codex: "codex", dsh: "deepseek", opencode: "opencode" };
const systemAvatarImages: Record<string, string> = {
  codex: "/agent-brands/codex-pixel.png",
  deepseek: "/agent-brands/deepseek-pixel.png",
  opencode: "/agent-brands/opencode-pixel.png",
};

export const BUILTIN_AGENT_AVATAR_IDS = ["codex", "deepseek", "opencode", "robot", "fox", "cat", "owl", "astronaut", "wizard", "panda", "frog", "penguin", "raccoon", "rabbit", "dragon", "sun", "moon", "earth", "mars", "mercury", "jupiter", "pig", "dog", "cow", "sheep", "horse", "snake", "rat", "chinese-dragon", "kucha", "black-hole", "neo", "red-blue-pills", "tibo"] as const;
const assets = import.meta.glob<string>("../assets/avatars/*.webp", { eager: true, query: "?url", import: "default" });

export function AgentAvatar({ avatarId, alt = "", className }: { avatarId?: string; alt?: string; className?: string }) {
  const id = BUILTIN_AGENT_AVATAR_IDS.find((id) => id === avatarId) ?? "robot";
  return <img src={systemAvatarImages[id] ?? assets[`../assets/avatars/${id}.webp`]} alt={alt} width={64} height={64} draggable={false} className={cn("object-cover [image-rendering:pixelated]", className)} />;
}

export function AgentAvatarPicker({ value, disabled, onChange }: { value: string; disabled?: boolean; onChange: (id: string) => void }) {
  const { t } = useTranslation("pages");
  const label = t("workspaceAgents.internal.avatar.label", { defaultValue: "Avatar" });
  const selectedId = BUILTIN_AGENT_AVATAR_IDS.find((id) => id === value) ?? "robot";
  return <fieldset disabled={disabled} className="space-y-2">
    <legend className="text-sm font-medium">{label}</legend>
    <div className="grid w-full max-w-xl grid-cols-[minmax(0,1fr)_7.5rem] gap-2">
      <div className="relative aspect-square min-w-0 overflow-hidden rounded-2xl">
        <AgentAvatar avatarId={selectedId} alt={t(`workspaceAgents.internal.avatar.${selectedId}`)} className="absolute inset-0 size-full" />
      </div>
      <div className="relative min-h-0">
        <div className="absolute inset-0">
          <ScrollArea scrollbarGutter>
            <div className="grid grid-cols-2 gap-2 p-1" role="group" aria-label={label}>
              {BUILTIN_AGENT_AVATAR_IDS.map((id) => <Button key={id} type="button" variant="outline" size="icon" aria-pressed={selectedId === id}
                aria-label={t(`workspaceAgents.internal.avatar.${id}`)} title={t(`workspaceAgents.internal.avatar.${id}`)}
                className={cn("relative aspect-square size-full overflow-hidden rounded-lg border-0 p-0 transition-opacity hover:opacity-80 motion-reduce:transition-none", selectedId === id && "ring-2 ring-[var(--color-focus-ring,var(--color-primary))] ring-offset-2 ring-offset-[var(--color-surface)]")}
                onClick={() => onChange(id)}>
                <AgentAvatar avatarId={id} className="size-full" />
                {selectedId === id ? <span className="absolute bottom-0 right-0 rounded-tl bg-[var(--color-primary)] p-0.5 text-[var(--color-on-primary)]"><Check size={10} aria-hidden="true" /></span> : null}
              </Button>)}
            </div>
          </ScrollArea>
        </div>
      </div>
    </div>
  </fieldset>;
}
