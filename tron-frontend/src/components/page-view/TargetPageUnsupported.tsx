import { ExternalLink } from "lucide-react";

interface TargetPageUnsupportedProps {
  onOpenInSystemBrowser: () => void;
}

export function TargetPageUnsupported({ onOpenInSystemBrowser }: TargetPageUnsupportedProps) {
  return (
    <div className="flex min-h-[560px] flex-auto items-center justify-center bg-[var(--color-elevated-surface-strong)] text-[var(--color-on-surface-variant)]">
      <div className="flex max-w-[420px] flex-col items-center gap-4 text-center">
        <ExternalLink className="size-8" />
        <div className="space-y-1"><p className="text-sm text-[var(--color-on-surface)]">当前页面不支持在内置浏览器中打开</p><p className="text-xs">请使用系统浏览器打开该页面。</p></div>
        <button className="inline-flex h-9 items-center gap-[12px] rounded-full border border-[var(--color-outline-variant)] px-4 text-sm text-[var(--color-on-surface)] transition-colors hover:bg-[var(--color-elevated-surface)]" type="button" onClick={onOpenInSystemBrowser}><ExternalLink className="size-4" />使用系统浏览器打开</button>
      </div>
    </div>
  );
}
