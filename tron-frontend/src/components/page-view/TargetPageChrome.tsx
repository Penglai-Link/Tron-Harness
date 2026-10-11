import type { FormEvent, FocusEvent, KeyboardEvent, RefObject } from "react";

import { useTranslation } from "react-i18next";

interface TargetPageChromeProps {
  addressEditing: boolean;
  addressFormRef: RefObject<HTMLFormElement | null>;
  addressInputRef: RefObject<HTMLInputElement | null>;
  addressValue: string;
  canGoBack: boolean;
  canGoForward: boolean;
  inspectionMode?: boolean;
  injectionEnabled: boolean;
  isDesktopHost: boolean;
  onAddressBlur: (event: FocusEvent<HTMLFormElement>) => void;
  onAddressChange: (value: string) => void;
  onAddressFocus: () => void;
  onAddressKeyDown: (event: KeyboardEvent<HTMLInputElement>) => void;
  onAddressSubmit: (event: FormEvent<HTMLFormElement>) => void;
  onBack: () => void;
  onForward: () => void;
  onInjectionToggle: () => void;
  onRefresh: () => void;
  onSubmitPointerDown: () => void;
}

export function TargetPageChrome({
  addressEditing,
  addressFormRef,
  addressInputRef,
  addressValue,
  canGoBack,
  canGoForward,
  inspectionMode = false,
  injectionEnabled,
  isDesktopHost,
  onAddressBlur,
  onAddressChange,
  onAddressFocus,
  onAddressKeyDown,
  onAddressSubmit,
  onBack,
  onForward,
  onInjectionToggle,
  onRefresh,
  onSubmitPointerDown,
}: TargetPageChromeProps) {
  const { t } = useTranslation(["common", "content"]);

  return (
    <div className="flex flex-shrink-0 min-h-14 flex-wrap gap-2 py-2 border-b border-b-gray-200 items-center justify-between bg-[var(--color-elevated-surface-strong)] px-4 relative">
      <div className="flex items-center gap-2 text-[var(--color-on-surface-variant)]">
        <button aria-label={t("actions.back", "Go back")} className="inline-flex items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)] disabled:opacity-40" disabled={!canGoBack} type="button" onClick={onBack}>{t("actions.back", "Go back")}</button>
        <button aria-label={t("actions.forward", "Go forward")} className="inline-flex items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)] disabled:opacity-40" disabled={!canGoForward} type="button" onClick={onForward}>{t("actions.forward", "Go forward")}</button>
        <button aria-label={t("actions.refresh", "Refresh frame")} className="inline-flex items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)]" type="button" onClick={onRefresh}>{t("actions.refresh", "Refresh frame")}</button>
      </div>
      <form autoComplete="off" ref={addressFormRef} className={`${addressEditing ? "border border-[var(--color-outline-variant)] bg-none" : "justify-center"} flex h-9 min-w-[220px] mx-2 w-1/2 flex-1 items-center gap-[12px] rounded-full px-4 text-sm text-[var(--color-on-surface)] md:flex-none`} onSubmit={onAddressSubmit} onBlur={onAddressBlur}>
        <input autoComplete="off" autoCapitalize="none" autoCorrect="off" placeholder={t('common:content.inputUrlTips')} aria-label="Target address" ref={addressInputRef} name="targetAddress" className={`${addressEditing ? "text-left" : ""} min-w-0 h-full flex-1 bg-transparent text-center text-sm text-[var(--color-on-surface)] outline-none`} spellCheck={false} value={addressValue} onChange={(event) => onAddressChange(event.target.value)} onFocus={onAddressFocus} onKeyDown={onAddressKeyDown} />
        {addressEditing ? (
          <button aria-label={t("actions.open", "Load target address")} className="inline-flex shrink-0 items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)]" type="submit" onPointerDown={onSubmitPointerDown}>{t("actions.open", "Load target address")}</button>
        ) : null}
      </form>
      {isDesktopHost && inspectionMode ? (
        <div className="flex items-center text-[var(--color-on-surface-variant)] text-sm">
          <button aria-label={injectionEnabled ? t("actions.disableInspection", "Disable inspection") : t("actions.enableInspection", "Enable inspection")} aria-pressed={injectionEnabled} className={`inline-flex items-center justify-center transition-colors hover:text-[var(--color-on-surface)] gap-1 ${injectionEnabled ? "text-[var(--color-on-surface)]" : "text-[var(--color-on-surface-variant)]"}`} type="button" onClick={onInjectionToggle}>
             {injectionEnabled ? t("actions.disableInspection", "Disable inspection") : t("actions.enableInspection", "Enable inspection")}
          </button>
        </div>
      ) : null}
    </div>
  );
}
