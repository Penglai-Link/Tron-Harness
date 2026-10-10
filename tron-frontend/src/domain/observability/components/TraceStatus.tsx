import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
export function TraceStatus({ status }: { status: string }) {
  const { t } = useTranslation("observability");
  return (
    <Badge
      variant={
        status === "Error"
          ? "error"
          : status === "Completed"
          ? "success"
          : status === "Running"
          ? "info"
          : "warning"
      }
    >
      {t(`status.${status}`, status)}
    </Badge>
  );
}
