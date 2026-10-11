import { useTranslation } from "react-i18next";
import { Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useToast } from "@/components/ui/toast";

export function TraceIdentifier({
  value,
  description,
  label,
  onClick,
  className,
}: {
  value: string;
  description?: string;
  label: string;
  onClick: () => void;
  className?: string;
}) {
  const { t } = useTranslation("observability");
  const toast = useToast();
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
      toast.success(t("copied"));
    } catch {
      toast.error(t("copyFailed"));
    }
  };
  return (
    <div className={`flex items-center gap-1 ${className || "w-44"}`}>
      <Tooltip>
        <TooltipTrigger
          render={
            <Button
              variant="link"
              className="h-auto min-w-0 flex-1 justify-start p-0 font-mono text-xs"
              onClick={onClick}
            />
          }
        >
          <span className="block truncate">{value || "—"}</span>
        </TooltipTrigger>
        <TooltipContent className="max-w-80 break-all">
          <div>
            {description && (
              <div className="mb-1 font-medium">{description}</div>
            )}
            <div className="font-mono">{value || "—"}</div>
          </div>
        </TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger render={
          <Button
            size="sm"
            variant="ghost"
            className="h-6 w-6 shrink-0 p-0"
            aria-label={`${t("copyId", { label })}: ${value}`}
            disabled={!value}
            onClick={() => void copy()}
          />
        }>
          <Copy className="size-3.5" aria-hidden="true" />
        </TooltipTrigger>
        <TooltipContent>{t("copyId", { label })}</TooltipContent>
      </Tooltip>
    </div>
  );
}
