import { useComposerDisabled } from "@eliteforge/agent-ui-composer-react";
import { useEffect, useRef, type RefObject } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { LoaderCircle, Mic, Square, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { fetchAutomaticRouters, automaticRouterQueryKey, AUTOMATIC_ROUTERS_CHANGED, routeHasCandidates } from "@/domain/model/api/automaticRouterApi";
import { useAppShellStore } from "@/store/appShellStore";
import { useAuthStore } from "@/store/authStore";
import { formatShortcut, hasSystemModifier, isEditableShortcutTarget, shortcutMatchesKeyboardEvent } from "@/shared/lib/keyboardShortcuts";
import { useSpeechInput } from "../hooks/useSpeechInput";

type SpeechTarget = { button: HTMLButtonElement; composer: RefObject<HTMLTextAreaElement | null>; floating: boolean };
const targets = new Set<SpeechTarget>();

export function ChatSpeechButton({ composer, active, scope, floating, onTranscript }: {
  composer: RefObject<HTMLTextAreaElement | null>; active: boolean; scope: string;
  floating: boolean; onTranscript: (text: string) => void;
}) {
  const disabled = useComposerDisabled();
  const { t } = useTranslation("pages");
  const user = useAuthStore(state => state.session?.user.userCode);
  const binding = useAppShellStore(state => state.shortcutBindings.toggleVoiceInput);
  const routes = useQuery({
    queryKey: [...automaticRouterQueryKey, user], queryFn: ({ signal }) => fetchAutomaticRouters(signal),
    enabled: active, staleTime: 0, refetchInterval: active ? 15000 : false,
  });
  const refetch = routes.refetch;
  useEffect(() => {
    const refresh = () => { if (active) void refetch(); };
    window.addEventListener(AUTOMATIC_ROUTERS_CHANGED, refresh);
    return () => window.removeEventListener(AUTOMATIC_ROUTERS_CHANGED, refresh);
  }, [active, refetch]);
  const available = !routes.isError && (routes.data?.some(route => route.kind === "SpeechRecognition" && routeHasCandidates(route)) ?? false);
  const speech = useSpeechInput({ active: active && available && !disabled, scope: `${user ?? ""}:${scope}`, onTranscript });
  const button = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!active || !available || disabled || !button.current) return;
    const target: SpeechTarget = { button: button.current, composer, floating };
    targets.add(target);
    const keydown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.repeat || event.isComposing) return;
      if (event.key === "Escape" && speech.state !== "idle") {
        event.preventDefault(); speech.cancel(); return;
      }
      if (!shortcutMatchesKeyboardEvent(binding, event)) return;
      if (isEditableShortcutTarget(event.target) && !hasSystemModifier(binding)) return;
      const visible = [...targets].filter(item => item.button.getClientRects().length > 0 && !item.composer.current?.disabled);
      const focused = visible.find(item => item.composer.current === document.activeElement || item.button === document.activeElement);
      if (!focused && isEditableShortcutTarget(event.target)) return;
      const selected = focused ?? visible.find(item => item.floating) ?? visible[0];
      if (selected !== target || target.button.disabled) return;
      event.preventDefault(); void speech.toggle();
    };
    window.addEventListener("keydown", keydown);
    return () => { targets.delete(target); window.removeEventListener("keydown", keydown); };
  }, [active, available, disabled, binding, composer, floating, speech.state, speech.toggle, speech.cancel]);
  if (!available) return null;
  const busy = speech.state === "requesting" || speech.state === "transcribing";
  const recording = speech.state === "recording";
  const label = t(`voiceInput.${recording ? "stop" : busy ? speech.state : "start"}`);
  const shortcut = formatShortcut(binding, /Mac|iPhone|iPad/i.test(navigator.userAgent));
  return <div className="flex shrink-0 items-center gap-1">
    <Tooltip>
      <TooltipTrigger render={<Button ref={button} type="button" variant="ghost" size="icon-sm" disabled={!active || disabled || busy}
        aria-label={label} aria-pressed={recording} onClick={() => { if (!composer.current?.disabled) void speech.toggle(); }}
        className={recording ? "text-[var(--color-error)]" : undefined} />}>
        {busy ? <LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" /> : recording ? <Square aria-hidden="true" className="size-4" /> : <Mic aria-hidden="true" className="size-4" />}
      </TooltipTrigger>
      <TooltipContent>{label}{shortcut ? ` (${shortcut})` : ""}</TooltipContent>
    </Tooltip>
    {speech.state !== "idle" && <>
      <span role="status" className="text-xs tabular-nums text-[var(--color-on-surface-variant)]">{recording ? t("voiceInput.recording", { seconds: speech.seconds }) : label}</span>
      <Button type="button" variant="ghost" size="icon-sm" aria-label={t("voiceInput.cancel")} title={t("voiceInput.cancel")} onClick={speech.cancel}><X aria-hidden="true" className="size-3.5" /></Button>
    </>}
  </div>;
}
