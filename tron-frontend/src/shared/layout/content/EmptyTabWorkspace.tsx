import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useChatCreationActions } from "@/shared/layout/chat";
import { formatShortcutKeys } from "@/shared/lib/keyboardShortcuts";
import { MotionReveal } from "@/shared/motion";
import { useAppShellStore } from "@/store/appShellStore";

export function EmptyTabWorkspace() {
  const { t } = useTranslation(["shell", "pages"]);
  const actions = useChatCreationActions();
  const bindings = useAppShellStore((state) => state.shortcutBindings);
  const mac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/i.test(navigator.userAgent);
  return (
    <section className="tron-workspace-start" aria-label={t("shell:workspaceStart.title")} data-testid="tron-empty-tab-workspace">
      <MotionReveal className="flex w-full max-w-[36rem] flex-col items-center text-center">
        <h1 className="text-[clamp(1.5rem,3vw,2.5rem)] font-semibold leading-tight tracking-tight">{t("shell:workspaceStart.title")}</h1>
        <p className="mt-3 text-base text-[var(--color-on-surface-variant)] sm:text-lg">{t("shell:workspaceStart.description")}</p>
        <Button className="mt-8 min-h-12 min-w-40 px-8 text-base" onClick={actions?.createQuickChat} disabled={!actions?.available}>
          {t("shell:workspaceStart.newChat")}
        </Button>
        <div className="mt-10 w-full space-y-3 text-left" data-testid="tron-empty-tab-shortcuts">
          {(["newTopic", "newDirectory"] as const).map((action) => (
            <Button
              key={action}
              variant="outline"
              className="h-auto min-h-14 w-full justify-between gap-3 whitespace-normal px-5 py-3 text-base"
              disabled={!actions?.available || (action === "newDirectory" && actions.addingDirectory)}
              onClick={action === "newTopic" ? actions?.createTopic : actions?.addDirectory}
            >
              <span>{t(`pages:settingsShortcuts.shortcuts.${action}.label`)}</span>
              <span className="flex shrink-0 gap-1">
                {(bindings[action] ? formatShortcutKeys(bindings[action], mac) : []).map((key, index) => (
                  <kbd key={index} className="min-w-6 rounded-md border border-[var(--color-card-border)] bg-[var(--color-surface-container-low)] px-1 py-0.5 text-center text-xs text-[var(--color-on-surface-variant)]">{key}</kbd>
                ))}
              </span>
            </Button>
          ))}
        </div>
      </MotionReveal>
    </section>
  );
}
