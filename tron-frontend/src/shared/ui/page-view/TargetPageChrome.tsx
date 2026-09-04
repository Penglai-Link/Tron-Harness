import type { FormEvent, FocusEvent, KeyboardEvent, RefObject } from "react";
import { ArrowLeft, ArrowRight, Check, MessageCirclePlus, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";

interface TargetPageChromeProps {
  addressEditing: boolean;
  addressFormRef: RefObject<HTMLFormElement | null>;
  addressInputRef: RefObject<HTMLInputElement | null>;
  addressValue: string;
  canGoBack: boolean;
  canGoForward: boolean;
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
    <div className="flex flex-shrink-0 h-14 border-b border-b-gray-200 items-center justify-between bg-[var(--color-elevated-surface-strong)] px-4 relative">
      <div className="flex items-center gap-5 text-[var(--color-on-surface-variant)]">
        <button aria-label="Go back" className="inline-flex items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)] disabled:opacity-40" disabled={!canGoBack} type="button" onClick={onBack}>
          <ArrowLeft className="size-[18px]" />
        </button>
        <button aria-label="Go forward" className="inline-flex items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)] disabled:opacity-40" disabled={!canGoForward} type="button" onClick={onForward}>
          <ArrowRight className="size-[18px]" />
        </button>
        <button aria-label="Refresh frame" className="inline-flex items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)]" type="button" onClick={onRefresh}>
          <RefreshCw className="size-[18px]" />
        </button>
      </div>
      <form autoComplete="off" ref={addressFormRef} className={`${addressEditing ? "border border-[var(--color-outline-variant)] bg-none" : "justify-center"} flex h-9 min-w-[220px] mx-2 w-1/2 flex-1 items-center gap-[12px] rounded-full px-4 text-sm text-[var(--color-on-surface)] md:flex-none absolute -translate-1/2 left-1/2 top-1/2`} onSubmit={onAddressSubmit} onBlur={onAddressBlur}>
        <input autoComplete="off" autoCapitalize="none" autoCorrect="off" placeholder={t('common:content.inputUrlTips')} aria-label="Target address" ref={addressInputRef} name="targetAddress" className={`${addressEditing ? "text-left" : ""} min-w-0 h-full flex-1 bg-transparent text-center text-sm text-[var(--color-on-surface)] outline-none`} spellCheck={false} value={addressValue} onChange={(event) => onAddressChange(event.target.value)} onFocus={onAddressFocus} onKeyDown={onAddressKeyDown} />
        {addressEditing ? (
          <button aria-label="Load target address" className="inline-flex shrink-0 items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)]" type="submit" onPointerDown={onSubmitPointerDown}>
            <Check className="size-[16px]" />
          </button>
        ) : null}
      </form>
      {isDesktopHost ? (
        <div className="flex items-center text-[var(--color-on-surface-variant)] text-sm">
          <button aria-label={injectionEnabled ? "Disable inspection" : "Enable inspection"} aria-pressed={injectionEnabled} className={`inline-flex items-center justify-center transition-colors hover:text-[var(--color-on-surface)] gap-1 ${injectionEnabled ? "text-[var(--color-on-surface)]" : "text-[var(--color-on-surface-variant)]"}`} type="button" onClick={onInjectionToggle}>
            <MessageCirclePlus className="size-[18px]" /> {injectionEnabled && "正在注释"}
          </button>
        </div>
      ) : null}
    </div>
  );
}
