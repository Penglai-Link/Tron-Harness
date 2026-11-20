import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useChatCreationActions } from "@/shared/layout/chat";
import { ShortcutHint } from "@/shared/shortcuts/ShortcutHints";
import { MotionReveal } from "@/shared/motion";
import { useAppShellStore } from "@/store/appShellStore";

export function EmptyTabWorkspace() {
  const { t } = useTranslation(["shell", "pages"]);
  const actions = useChatCreationActions();
  const bindings = useAppShellStore((state) => state.shortcutBindings);
  const mac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/i.test(navigator.userAgent);
  const newSessionBinding = bindings.newSession;
  return (
    <section className="tron-workspace-start" aria-label={t("shell:workspaceStart.title")} data-testid="tron-empty-tab-workspace">
      <MotionReveal className="flex w-full flex-col items-center text-center">
        <h1 className="w-fit max-w-[48rem] text-balance text-[clamp(1.5rem,3vw,2.5rem)] font-semibold leading-tight tracking-tight">{t("shell:workspaceStart.title")}</h1>
        <Button
          className="mt-8 min-h-12 min-w-40 gap-3 px-8 text-base"
          onClick={actions?.createQuickChat}
          disabled={!actions?.available}
          aria-keyshortcuts={newSessionBinding ? [
            newSessionBinding.modKey ? (mac ? "Meta" : "Control") : "",
            newSessionBinding.altKey ? "Alt" : "",
            newSessionBinding.shiftKey ? "Shift" : "",
            newSessionBinding.key,
          ].filter(Boolean).join("+") : undefined}
        >
          <span>{t("shell:workspaceStart.newChat")}</span>
          <ShortcutHint action="newSession" />
        </Button>
        <div className="mt-10 w-full max-w-[36rem] space-y-3 text-left" data-testid="tron-empty-tab-shortcuts">
          {(["newTopic", "newDirectory"] as const).map((action) => (
            <Button
              key={action}
              variant="outline"
              className="h-auto min-h-14 w-full justify-between gap-3 whitespace-normal px-5 py-3 text-base"
              disabled={!actions?.available || (action === "newDirectory" && actions.addingDirectory)}
              onClick={action === "newTopic" ? actions?.createTopic : actions?.addDirectory}
            >
              <span>{t(`pages:settingsShortcuts.shortcuts.${action}.label`)}</span>
              <ShortcutHint action={action} />
            </Button>
          ))}
        </div>
      </MotionReveal>
    </section>
  );
}
