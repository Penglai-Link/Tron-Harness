import { Fragment, useEffect, useState } from "react";
import { Keyboard, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  formatShortcut,
  beginKeyboardShortcutCapture,
  shellShortcutActions,
  shortcutEquals,
  shortcutFromKeyboardEvent,
  type ShellShortcutAction,
} from "@/shared/lib/keyboardShortcuts";
import { useAppShellStore } from "@/store/appShellStore";
import {
  Item,
  ItemContent,
  ItemTitle,
  ItemDescription,
  ItemActions,
} from "@/components/ui/item";
import { Separator } from "@/components/ui/separator";
import { PageHeader, SurfaceCard } from "@/shared";

export function SettingsShortcutsPage() {
  const { t } = useTranslation("pages");
  const shortcutBindings = useAppShellStore((state) => state.shortcutBindings);
  const setShortcutBinding = useAppShellStore(
    (state) => state.setShortcutBinding,
  );
  const [capturingAction, setCapturingAction] =
    useState<ShellShortcutAction | null>(null);
  const [captureError, setCaptureError] = useState("");
  const pageT = (key: string, options?: Record<string, unknown>) =>
    t(`settingsShortcuts.${key}`, options);
  const useMacSymbols =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/i.test(navigator.userAgent);

  useEffect(() => {
    if (!capturingAction) return;

    const endKeyboardCapture = beginKeyboardShortcutCapture();

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopImmediatePropagation();

      if (event.key === "Escape") {
        setCapturingAction(null);
        setCaptureError("");
        return;
      }
      if (
        !event.altKey &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.shiftKey &&
        (event.key === "Backspace" || event.key === "Delete")
      ) {
        setShortcutBinding(capturingAction, null);
        setCapturingAction(null);
        setCaptureError("");
        return;
      }

      const binding = shortcutFromKeyboardEvent(event);
      if (!binding) {
        setCaptureError(pageT("messages.modifierRequired"));
        return;
      }

      const conflict = shellShortcutActions.find(
        (action) =>
          action !== capturingAction &&
          shortcutEquals(shortcutBindings[action], binding),
      );
      if (conflict) {
        setCaptureError(
          pageT("messages.conflict", {
            action: pageT(`shortcuts.${conflict}.label`),
          }),
        );
        return;
      }

      setShortcutBinding(capturingAction, binding);
      setCapturingAction(null);
      setCaptureError("");
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      endKeyboardCapture();
    };
  }, [capturingAction, pageT, setShortcutBinding, shortcutBindings]);

  function beginCapture(action: ShellShortcutAction) {
    setCapturingAction(action);
    setCaptureError("");
  }

  return (
    <div className="space-y-6">
      <PageHeader title={pageT("title")} />

      <SurfaceCard className="overflow-hidden p-0">
        <div className="border-b border-[var(--color-outline-variant)] px-2 py-4">
          <h2 className="text-sm font-semibold text-[var(--color-on-surface)]">
            {pageT("supportedTitle")}
          </h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
            {pageT("configureHint")}
          </p>
          {captureError ? (
            <p
              aria-live="polite"
              className="mt-2 text-sm text-[var(--color-error)]"
            >
              {captureError}
            </p>
          ) : null}
        </div>

        <div className="overflow-x-auto">
          <div className="min-w-[760px] divide-y divide-[var(--color-outline-variant)]">
            {shellShortcutActions.map((action, index) => {
              const binding = shortcutBindings[action];
              const isCapturing = capturingAction === action;
              const label = pageT(`shortcuts.${action}.label`);

              return (
                <Fragment key={action}>
                  <Item size="xs">
                    <ItemContent>
                      <ItemTitle>{label}</ItemTitle>
                      <ItemDescription>
                        {pageT(`shortcuts.${action}.description`)}
                      </ItemDescription>
                    </ItemContent>
                    <ItemActions>
                      <Button
                        aria-label={pageT("actions.configure", { label })}
                        className="min-w-[156px] justify-start font-mono"
                        size="sm"
                        type="button"
                        variant={isCapturing ? "secondary" : "outline"}
                        onClick={() => beginCapture(action)}
                      >
                        <Keyboard aria-hidden="true" size={14} />
                        {isCapturing
                          ? pageT("capturing")
                          : binding
                          ? formatShortcut(binding, useMacSymbols)
                          : pageT("unassigned")}
                      </Button>
                      {binding ? (
                        <Button
                          aria-label={pageT("actions.clear", { label })}
                          size="icon-sm"
                          type="button"
                          variant="ghost"
                          onClick={() => {
                            setShortcutBinding(action, null);
                            if (capturingAction === action) {
                              setCapturingAction(null);
                            }
                          }}
                        >
                          <X aria-hidden="true" size={15} />
                        </Button>
                      ) : null}
                    </ItemActions>
                  </Item>
                  {index < shellShortcutActions.length - 1 ? (
                    <Separator />
                  ) : null}
                </Fragment>
              );
            })}
          </div>
        </div>
      </SurfaceCard>
    </div>
  );
}
