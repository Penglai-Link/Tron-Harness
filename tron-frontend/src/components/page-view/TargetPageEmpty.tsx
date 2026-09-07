import { Globe } from "lucide-react";
import { type RefObject } from "react";
import { useTranslation } from "react-i18next";

export function TargetPageEmpty({
  targetViewportRef,
}: {
  targetViewportRef: RefObject<HTMLDivElement | null>;
}) {
  const { t } = useTranslation(["common", "content"]);
  return (
    <div
      ref={targetViewportRef}
      className="h-full bg-[var(--color-elevated-surface-strong)] opacity-40 flex items-center justify-center flex-col"
    >
      <Globe size={30} className="mb-2" />
      {t('common:content.openUrl')}
    </div>
  );
}
