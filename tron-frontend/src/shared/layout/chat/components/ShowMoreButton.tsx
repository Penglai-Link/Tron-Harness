import { MoreHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/shared/lib/utils";

export function ShowMoreButton({ className, onClick }: { className?: string; onClick: () => void }) {
  const { t } = useTranslation("shell");
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex h-6 w-fit items-center gap-1 rounded px-1.5 text-xs text-sidebar-foreground/55 transition-colors hover:text-sidebar-foreground focus-visible:outline-2 focus-visible:outline-ring motion-reduce:transition-none",
        className
      )}
    >
      <MoreHorizontal size={12} className="shrink-0 opacity-70" aria-hidden="true" />
      {t("chatSidebar.showMore")}
    </button>
  );
}